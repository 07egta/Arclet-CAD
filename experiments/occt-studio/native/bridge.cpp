// Original adapter code. OCCT is a separately licensed, dynamically linked dependency.
// No C++ exception may cross the C ABI. All shape ownership stays behind opaque handles.
#include <Standard_Version.hxx>
#include <Standard_Failure.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepAlgoAPI_Fuse.hxx>
#include <BRepAlgoAPI_Cut.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_Section.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepGProp.hxx>
#include <BRepBndLib.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepTools.hxx>
#include <BRep_Tool.hxx>
#include <BRep_Builder.hxx>
#include <BRepAdaptor_Curve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <GeomAPI_Interpolate.hxx>
#include <GCPnts_QuasiUniformDeflection.hxx>
#include <GProp_GProps.hxx>
#include <TColgp_Array1OfPnt.hxx>
#include <TColgp_HArray1OfPnt.hxx>
#include <TColStd_Array1OfReal.hxx>
#include <TColStd_Array1OfInteger.hxx>
#include <TopExp.hxx>
#include <TopExp_Explorer.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopTools_ListOfShape.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <Poly_Triangulation.hxx>
#include <Bnd_Box.hxx>
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#include <StlAPI_Writer.hxx>
#include <gp_Circ.hxx>
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Trsf.hxx>
#include <gp_Vec.hxx>
#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

struct Shape { TopoDS_Shape value; explicit Shape(TopoDS_Shape s) : value(std::move(s)) {} };
struct Bytes { std::string value; };
struct Mesh { std::vector<double> positions, normals, lines; std::vector<uint32_t> indices; };
struct MeshView { const double *positions, *normals, *lines; const uint32_t *indices; size_t vertices, triangles, segments; };
struct Stats { double volume, area, length, bounds[6]; uint64_t faces, edges, solids; };
static constexpr size_t MAX_VERTICES = 2000000, MAX_TRIANGLES = 2000000, MAX_SEGMENTS = 1000000;
static void message(char* out, size_t cap, const char* text) noexcept {
    if (!out || cap == 0) return;
    if (!text) text = "Open CASCADE failure";
    std::strncpy(out, text, cap - 1); out[cap - 1] = 0;
}
template<class F> auto protect(char* err, size_t cap, F f) noexcept -> decltype(f()) {
    if (err && cap) err[0] = 0;
    try { return f(); }
    catch (const Standard_Failure& e) { message(err, cap, e.GetMessageString()); }
    catch (const std::exception& e) { message(err, cap, e.what()); }
    catch (...) { message(err, cap, "Unknown native exception"); }
    return {};
}
static void require(bool ok, const char* why) { if (!ok) throw std::runtime_error(why); }
static const TopoDS_Shape& shape(const Shape* p) { require(p && !p->value.IsNull(), "Missing shape"); return p->value; }
static Shape* checked(TopoDS_Shape s) {
    require(!s.IsNull(), "Operation returned a null shape");
    require(BRepCheck_Analyzer(s).IsValid(), "Operation returned invalid B-rep topology");
    return new Shape(std::move(s));
}
static gp_Pnt point(const double* p) { return {p[0],p[1],p[2]}; }
static TopoDS_Shape profile(const TopoDS_Shape& s) {
    if (s.ShapeType() == TopAbs_WIRE && s.Closed()) {
        BRepBuilderAPI_MakeFace face(TopoDS::Wire(s), true);
        require(face.IsDone(), "Closed profile must be planar"); return face.Face();
    }
    require(s.ShapeType() == TopAbs_WIRE || s.ShapeType() == TopAbs_FACE, "Expected a wire or face profile");
    return s;
}
template<class Op> static Shape* boolean_op(const Shape* a, const Shape* b) {
    Op op; TopTools_ListOfShape aa, bb; aa.Append(shape(a)); bb.Append(shape(b));
    op.SetArguments(aa); op.SetTools(bb); op.SetNonDestructive(true); op.SetRunParallel(false); op.Build();
    require(op.IsDone() && !op.HasErrors(), "Boolean operation failed");
    TopExp_Explorer contents(op.Shape(), TopAbs_VERTEX);
    require(contents.More(), "Boolean result is empty");
    return checked(op.Shape());
}
extern "C" {
const char* ac_version() noexcept { return OCC_VERSION_COMPLETE; }
void ac_shape_free(Shape* p) noexcept { try { delete p; } catch (...) {} }
void ac_bytes_free(Bytes* p) noexcept { try { delete p; } catch (...) {} }
void ac_mesh_free(Mesh* p) noexcept { try { delete p; } catch (...) {} }
const char* ac_bytes_data(const Bytes* p) noexcept { return p ? p->value.data() : nullptr; }
size_t ac_bytes_size(const Bytes* p) noexcept { return p ? p->value.size() : 0; }
Shape* ac_op(int op, const Shape* a, const Shape* b, const double* v, size_t n,
             const Shape* const* group, size_t count, char* err, size_t cap) noexcept {
    return protect(err, cap, [&]() -> Shape* {
        require(n == 0 || v, "Missing numeric data");
        for (size_t i=0; i<n; ++i) require(std::isfinite(v[i]) && std::abs(v[i])<=1e8, "Invalid numeric data");
        auto size = [&](size_t expected){ require(n == expected, "Incorrect parameter count"); };
        switch (op) {
        case 1: size(6); require(v[3]>1e-7 && v[4]>1e-7 && v[5]>1e-7,"Box dimensions must be positive"); return checked(BRepPrimAPI_MakeBox(point(v),v[3],v[4],v[5]).Shape());
        case 2: size(4); require(v[3]>1e-7,"Radius must be positive"); return checked(BRepPrimAPI_MakeSphere(point(v),v[3]).Shape());
        case 3: size(5); require(v[3]>1e-7 && v[4]>1e-7,"Radius and height must be positive"); return checked(BRepPrimAPI_MakeCylinder(gp_Ax2(point(v),gp_Dir(0,0,1)),v[3],v[4]).Shape());
        case 4: size(6); require(v[3]>=0 && v[4]>=0 && std::max(v[3],v[4])>1e-7 && std::abs(v[3]-v[4])>1e-7 && v[5]>1e-7,"Invalid cone dimensions"); return checked(BRepPrimAPI_MakeCone(gp_Ax2(point(v),gp_Dir(0,0,1)),v[3],v[4],v[5]).Shape());
        case 5: size(5); require(v[4]>1e-7 && v[3]>v[4],"Torus requires major radius > minor radius > 0"); return checked(BRepPrimAPI_MakeTorus(gp_Ax2(point(v),gp_Dir(0,0,1)),v[3],v[4]).Shape());
        case 6: { size(4); require(v[3]>1e-7,"Radius must be positive"); BRepBuilderAPI_MakeEdge e(gp_Circ(gp_Ax2(point(v),gp_Dir(0,0,1)),v[3])); return checked(BRepBuilderAPI_MakeWire(e.Edge()).Wire()); }
        case 7: {
            require(n>=7 && (n-1)%3==0 && n<=769,"Polyline needs 2..256 points");
            BRepBuilderAPI_MakePolygon p; for(size_t i=1;i<n;i+=3) p.Add(point(v+i));
            if(v[0]!=0) p.Close(); require(p.IsDone(),"Polyline construction failed"); return checked(p.Wire());
        }
        case 8: {
            require(n>=9 && (n-1)%4==0 && n<=1025,"Invalid NURBS controls");
            int degree=static_cast<int>(v[0]), np=static_cast<int>((n-1)/4);
            require(v[0]==degree && degree>=1 && degree<=12 && np>degree,"Invalid NURBS degree");
            TColgp_Array1OfPnt poles(1,np); TColStd_Array1OfReal weights(1,np);
            for(int i=1;i<=np;++i) { size_t k=1+4*(i-1); poles.SetValue(i,point(v+k)); require(v[k+3]>1e-9 && v[k+3]<=1e6,"Invalid NURBS weight"); weights.SetValue(i,v[k+3]); }
            int nk=np-degree+1; TColStd_Array1OfReal knots(1,nk); TColStd_Array1OfInteger mult(1,nk);
            for(int i=1;i<=nk;++i){knots.SetValue(i,static_cast<double>(i-1)/(nk-1)); mult.SetValue(i,(i==1||i==nk)?degree+1:1);}
            Handle(Geom_BSplineCurve) c=new Geom_BSplineCurve(poles,weights,knots,mult,degree,false);
            return checked(BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(c).Edge()).Wire());
        }
        case 9: {
            require(n>=6 && n%3==0 && n<=768,"Curve needs 2..256 points");
            Handle(TColgp_HArray1OfPnt) points=new TColgp_HArray1OfPnt(1,static_cast<int>(n/3));
            for(int i=1;i<=points->Length();++i) points->SetValue(i,point(v+3*(i-1)));
            GeomAPI_Interpolate curve(points,false,1e-7); curve.Perform(); require(curve.IsDone(),"Curve interpolation failed");
            return checked(BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(curve.Curve()).Edge()).Wire());
        }
        case 10: size(3); require(gp_Vec(v[0],v[1],v[2]).Magnitude()>1e-7,"Extrusion vector is zero"); return checked(BRepPrimAPI_MakePrism(profile(shape(a)),gp_Vec(v[0],v[1],v[2]),true).Shape());
        case 11: size(7); require(std::abs(v[6])>1e-7 && std::abs(v[6])<=360,"Revolve angle out of range"); return checked(BRepPrimAPI_MakeRevol(profile(shape(a)),gp_Ax1(point(v),gp_Dir(v[3],v[4],v[5])),v[6]*3.14159265358979323846/180,true).Shape());
        case 12: {
            size(1); require(group && count>=2 && count<=64,"Loft needs 2..64 wires");
            BRepOffsetAPI_ThruSections loft(v[0]!=0,false,1e-7); loft.CheckCompatibility(true);
            for(size_t i=0;i<count;++i){const auto& s=shape(group[i]); require(s.ShapeType()==TopAbs_WIRE,"Loft sections must be wires"); require(v[0]==0 || s.Closed(),"Solid loft requires closed sections"); loft.AddWire(TopoDS::Wire(s));}
            loft.Build(); require(loft.IsDone(),"Loft construction failed"); return checked(loft.Shape());
        }
        case 20: size(0); return boolean_op<BRepAlgoAPI_Fuse>(a,b);
        case 21: size(0); return boolean_op<BRepAlgoAPI_Cut>(a,b);
        case 22: size(0); return boolean_op<BRepAlgoAPI_Common>(a,b);
        case 23: case 24: {
            require(n>=1 && n<=1001 && v[0]>1e-7,"Invalid edge treatment");
            TopTools_IndexedMapOfShape edges; TopExp::MapShapes(shape(a),TopAbs_EDGE,edges);
            std::vector<int> ids;
            if(n==1){for(int i=1;i<=edges.Extent();++i) ids.push_back(i);}
            else for(size_t i=1;i<n;++i){int id=static_cast<int>(v[i]); require(v[i]==id && id>=1 && id<=edges.Extent(),"Edge index out of bounds"); if(std::find(ids.begin(),ids.end(),id)==ids.end()) ids.push_back(id);}
            require(!ids.empty(),"Shape has no edges");
            if(op==23){BRepFilletAPI_MakeFillet f(shape(a)); for(int id:ids) f.Add(v[0],TopoDS::Edge(edges(id))); f.Build(); require(f.IsDone(),"Fillet failed; try a smaller radius or fewer edges"); return checked(f.Shape());}
            BRepFilletAPI_MakeChamfer c(shape(a)); for(int id:ids) c.Add(v[0],TopoDS::Edge(edges(id))); c.Build(); require(c.IsDone(),"Chamfer failed"); return checked(c.Shape());
        }
        case 25: case 26: case 27: case 28: {
            gp_Trsf t;
            if(op==25){size(3);t.SetTranslation(gp_Vec(v[0],v[1],v[2]));}
            if(op==26){size(7);t.SetRotation(gp_Ax1(point(v),gp_Dir(v[3],v[4],v[5])),v[6]*3.14159265358979323846/180);}
            if(op==27){size(4);require(v[3]>1e-7 && v[3]<=1e4,"Scale factor out of range");t.SetScale(point(v),v[3]);}
            if(op==28){size(6);t.SetMirror(gp_Ax2(point(v),gp_Dir(v[3],v[4],v[5])));}
            return checked(BRepBuilderAPI_Transform(shape(a),t,true).Shape());
        }
        case 30: {
            require(group && count>0 && count<=2048,"Compound requires 1..2048 shapes");
            TopoDS_Compound c; BRep_Builder builder; builder.MakeCompound(c); for(size_t i=0;i<count;++i) builder.Add(c,shape(group[i])); return checked(c);
        }
        case 31: { size(0); BRepAlgoAPI_Section s(shape(a),shape(b),false); s.Build();require(s.IsDone(),"Section failed");return checked(s.Shape()); }
        default: throw std::runtime_error("Unknown native operation");
        }
    });
}
int ac_stats(const Shape* p, Stats* out, char* err, size_t cap) noexcept {
    return protect(err,cap,[&](){require(out,"Missing output");const auto& s=shape(p);*out={};
        GProp_GProps v,a,l; BRepGProp::VolumeProperties(s,v);BRepGProp::SurfaceProperties(s,a);BRepGProp::LinearProperties(s,l);
        out->volume=v.Mass();out->area=a.Mass();out->length=l.Mass();
        Bnd_Box box;BRepBndLib::AddOptimal(s,box,false,false);require(!box.IsVoid(),"Shape has no bounds");
        box.Get(out->bounds[0],out->bounds[1],out->bounds[2],out->bounds[3],out->bounds[4],out->bounds[5]);
        TopTools_IndexedMapOfShape f,e,so;TopExp::MapShapes(s,TopAbs_FACE,f);TopExp::MapShapes(s,TopAbs_EDGE,e);TopExp::MapShapes(s,TopAbs_SOLID,so);
        out->faces=f.Extent();out->edges=e.Extent();out->solids=so.Extent();return 1;});
}
Bytes* ac_brep(const Shape* p,char* err,size_t cap) noexcept {
    return protect(err,cap,[&]()->Bytes*{std::ostringstream out;BRepTools::Write(shape(p),out);require(out.good(),"BREP serialization failed");return new Bytes{out.str()};});
}
Shape* ac_read(const char* data,size_t len,int format,char* err,size_t cap) noexcept {
    return protect(err,cap,[&]()->Shape*{require(data && len>0 && len<=67108864,"Invalid input length");std::string text(data,len);
        if(format==1){std::istringstream in(text);TopoDS_Shape s;BRep_Builder b;BRepTools::Read(s,in,b);return checked(s);}
        require(format==2 && text.find('\0')==std::string::npos,"Invalid STEP path");STEPControl_Reader r;require(r.ReadFile(text.c_str())==IFSelect_RetDone,"Cannot read STEP file");require(r.TransferRoots()>0,"STEP file has no transferable roots");return checked(r.OneShape());});
}
int ac_write(const Shape* p,const char* path,int format,char* err,size_t cap) noexcept {
    return protect(err,cap,[&](){require(path && *path,"Missing export path");const auto& s=shape(p);
        if(format==1){STEPControl_Writer w;require(w.Transfer(s,STEPControl_AsIs)==IFSelect_RetDone,"STEP transfer failed");require(w.Write(path)==IFSelect_RetDone,"STEP write failed");}
        else {require(format==2,"Unknown export format");BRepBuilderAPI_Copy copy(s,true,false);auto c=copy.Shape();BRepMesh_IncrementalMesh mesher(c,0.05,false,0.25,false);require(mesher.IsDone(),"STL meshing failed");StlAPI_Writer w;w.ASCIIMode()=false;require(w.Write(c,path),"STL write failed");}return 1;});
}
Mesh* ac_mesh(const Shape* p,double deflection,char* err,size_t cap) noexcept {
    return protect(err,cap,[&]()->Mesh*{require(std::isfinite(deflection) && deflection>=1e-4 && deflection<=1e4,"Invalid mesh deflection");
        auto result=std::make_unique<Mesh>();BRepBuilderAPI_Copy copy(shape(p),true,false);auto s=copy.Shape();
        BRepMesh_IncrementalMesh mesher(s,deflection,false,0.25,false);require(mesher.IsDone(),"Tessellation failed");
        for(TopExp_Explorer it(s,TopAbs_FACE);it.More();it.Next()){
            auto face=TopoDS::Face(it.Current());TopLoc_Location loc;auto tri=BRep_Tool::Triangulation(face,loc);if(tri.IsNull())continue;
            size_t base=result->positions.size()/3;require(base+tri->NbNodes()<=MAX_VERTICES && result->indices.size()/3+tri->NbTriangles()<=MAX_TRIANGLES,"Tessellation exceeds safety budget; increase deflection");
            for(int i=1;i<=tri->NbNodes();++i){auto pnt=tri->Node(i).Transformed(loc.Transformation());result->positions.insert(result->positions.end(),{pnt.X(),pnt.Y(),pnt.Z()});result->normals.insert(result->normals.end(),{0,0,0});}
            bool reverse=face.Orientation()==TopAbs_REVERSED;
            for(int i=1;i<=tri->NbTriangles();++i){int a,b,c;tri->Triangle(i).Get(a,b,c);if(reverse)std::swap(b,c);
                uint32_t ia=static_cast<uint32_t>(base+a-1),ib=static_cast<uint32_t>(base+b-1),ic=static_cast<uint32_t>(base+c-1);
                result->indices.insert(result->indices.end(),{ia,ib,ic});
                gp_Vec ab(point(result->positions.data()+ia*3),point(result->positions.data()+ib*3));gp_Vec ac(point(result->positions.data()+ia*3),point(result->positions.data()+ic*3));auto normal=ab.Crossed(ac);
                for(uint32_t k:{ia,ib,ic}){result->normals[k*3]+=normal.X();result->normals[k*3+1]+=normal.Y();result->normals[k*3+2]+=normal.Z();}}
        }
        for(size_t i=0;i<result->normals.size();i+=3){double x=result->normals[i],y=result->normals[i+1],z=result->normals[i+2],len=std::sqrt(x*x+y*y+z*z);if(len>1e-20){result->normals[i]/=len;result->normals[i+1]/=len;result->normals[i+2]/=len;}}
        TopTools_IndexedMapOfShape edges;TopExp::MapShapes(s,TopAbs_EDGE,edges);
        for(int i=1;i<=edges.Extent();++i){auto e=TopoDS::Edge(edges(i));if(BRep_Tool::Degenerated(e))continue;BRepAdaptor_Curve curve(e);GCPnts_QuasiUniformDeflection sample(curve,deflection);
            require(sample.IsDone(),"Edge sampling failed");require(sample.NbPoints()<=100000,"Edge sampling exceeds budget");
            for(int j=2;j<=sample.NbPoints();++j){require(result->lines.size()/6<MAX_SEGMENTS,"Edge display exceeds budget");auto a=sample.Value(j-1),b=sample.Value(j);result->lines.insert(result->lines.end(),{a.X(),a.Y(),a.Z(),b.X(),b.Y(),b.Z()});}}
        return result.release();});
}
MeshView ac_mesh_view(const Mesh* p) noexcept {
    if(!p)return {};return {p->positions.data(),p->normals.data(),p->lines.data(),p->indices.data(),p->positions.size()/3,p->indices.size()/3,p->lines.size()/6};
}
}
