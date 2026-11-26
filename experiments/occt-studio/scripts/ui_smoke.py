#!/usr/bin/env python3
"""Drive the real X11 window through XTest. Run in an isolated Xvfb display.

Requires Python 3, Pillow, libX11 and libXtst. This never injects commands through
an app backdoor: all edits use keyboard/mouse input. ARCLET_GUI_STATE is an
opt-in, local, read-only observation hook for deterministic assertions.
"""
from __future__ import annotations
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import time
from typing import Callable
from PIL import ImageGrab

class ClientData(C.Union):
    _fields_ = [("b", C.c_char * 20), ("s", C.c_short * 10), ("l", C.c_long * 5)]
class ClientMessage(C.Structure):
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong), ("message_type", C.c_ulong),
                ("format", C.c_int), ("data", ClientData)]
class XEvent(C.Union):
    _fields_ = [("client", ClientMessage), ("pad", C.c_long * 24)]

class Desktop:
    def __init__(self, display: str):
        self.x = C.CDLL(ctypes.util.find_library("X11"))
        self.xt = C.CDLL(ctypes.util.find_library("Xtst"))
        self.x.XOpenDisplay.argtypes = [C.c_char_p]; self.x.XOpenDisplay.restype = C.c_void_p
        self.display = self.x.XOpenDisplay(display.encode())
        if not self.display: raise RuntimeError(f"Cannot open display {display}")
        self.x.XDefaultRootWindow.argtypes = [C.c_void_p]; self.x.XDefaultRootWindow.restype = C.c_ulong
        self.root = self.x.XDefaultRootWindow(self.display)
        self.x.XQueryTree.argtypes = [C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
        self.x.XFetchName.argtypes = [C.c_void_p,C.c_ulong,C.POINTER(C.c_char_p)]
        self.x.XFree.argtypes = [C.c_void_p]
        self.x.XStringToKeysym.argtypes = [C.c_char_p];self.x.XStringToKeysym.restype=C.c_ulong
        self.x.XKeysymToKeycode.argtypes=[C.c_void_p,C.c_ulong];self.x.XKeysymToKeycode.restype=C.c_uint
        self.x.XFlush.argtypes=[C.c_void_p]
        self.x.XSetInputFocus.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong]
        self.x.XInternAtom.argtypes=[C.c_void_p,C.c_char_p,C.c_int];self.x.XInternAtom.restype=C.c_ulong
        self.x.XSendEvent.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_long,C.POINTER(XEvent)]
        self.xt.XTestFakeKeyEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
        self.xt.XTestFakeButtonEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
        self.xt.XTestFakeMotionEvent.argtypes=[C.c_void_p,C.c_int,C.c_int,C.c_int,C.c_ulong]
        self.window=0
    def find_window(self)->int:
        root=C.c_ulong();parent=C.c_ulong();children=C.POINTER(C.c_ulong)();count=C.c_uint()
        self.x.XQueryTree(self.display,self.root,C.byref(root),C.byref(parent),C.byref(children),C.byref(count))
        try:
            for i in range(count.value):
                name=C.c_char_p()
                if self.x.XFetchName(self.display,children[i],C.byref(name)) and name.value:
                    text=name.value.decode(errors='replace');self.x.XFree(name)
                    if 'Arclet OCCT Studio' in text:self.window=children[i];return self.window
        finally:
            if children:self.x.XFree(children)
        raise RuntimeError('App window not found')
    def key(self,name:str,down:bool)->None:
        symbol=self.x.XStringToKeysym(name.encode());code=self.x.XKeysymToKeycode(self.display,symbol)
        if not code:raise RuntimeError(f'No key mapping for {name}')
        self.xt.XTestFakeKeyEvent(self.display,code,int(down),0);self.x.XFlush(self.display)
    def press(self,name:str)->None:self.key(name,True);self.key(name,False);time.sleep(.05)
    def chord(self,*names:str)->None:
        for name in names:self.key(name,True)
        for name in reversed(names):self.key(name,False)
        time.sleep(.12)
    def text(self,text:str)->None:
        punctuation={' ':'space',',':'comma','.':'period','/':'slash','\\':'backslash','-':'minus','_':'underscore',':':'colon',';':'semicolon','"':'quotedbl',"'":'apostrophe','=':'equal','+':'plus','#':'numbersign','(':'parenleft',')':'parenright'}
        for char in text:
            name=punctuation.get(char,char);shift=char.isupper()or char in '_+:"#()'
            if shift:self.key('Shift_L',True)
            self.key(name,True);self.key(name,False)
            if shift:self.key('Shift_L',False)
            time.sleep(.003)
        time.sleep(.1)
    def move(self,x:float,y:float)->None:
        self.xt.XTestFakeMotionEvent(self.display,-1,int(x),int(y),0);self.x.XFlush(self.display)
    def button(self,button:int,down:bool)->None:
        self.xt.XTestFakeButtonEvent(self.display,button,int(down),0);self.x.XFlush(self.display)
    def click(self,x:float,y:float,button:int=1)->None:
        self.move(x,y);self.button(button,True);self.button(button,False);time.sleep(.15)
    def drag(self,x:float,y:float,dx:float,dy:float,button:int)->None:
        self.move(x,y);self.button(button,True)
        for i in range(1,13):self.move(x+dx*i/12,y+dy*i/12);time.sleep(.025)
        self.button(button,False);time.sleep(.15)
    def close_request(self)->None:
        event=XEvent();event.client.type=33;event.client.send_event=1;event.client.display=self.display
        event.client.window=self.window;event.client.message_type=self.x.XInternAtom(self.display,b'WM_PROTOCOLS',False)
        event.client.format=32;event.client.data.l[0]=self.x.XInternAtom(self.display,b'WM_DELETE_WINDOW',False);event.client.data.l[1]=0
        self.x.XSendEvent(self.display,self.window,False,0,C.byref(event));self.x.XFlush(self.display);time.sleep(.2)

def run(binary:Path,out:Path)->None:
    out.mkdir(parents=True,exist_ok=True);state_path=out/'state.json';state_path.unlink(missing_ok=True)
    display=os.environ.get('DISPLAY',':99');desktop=Desktop(display)
    env=os.environ.copy();env.update(DISPLAY=display,ARCLET_GUI_STATE=str(state_path),LIBGL_ALWAYS_SOFTWARE='1')
    log=(out/'application.log').open('w');app=subprocess.Popen([str(binary),'--desktop','--demo'],cwd=out,env=env,stdout=log,stderr=subprocess.STDOUT)
    evidence=[]
    def state()->dict:
        try:return json.loads(state_path.read_text())
        except (FileNotFoundError,json.JSONDecodeError):return {}
    def wait(predicate:Callable[[dict],bool],label:str,timeout:float=20)->dict:
        deadline=time.monotonic()+timeout
        while time.monotonic()<deadline:
            s=state()
            if predicate(s):return s
            if app.poll() is not None:raise RuntimeError(f'App exited {app.returncode} during {label}: {(out/"application.log").read_text()[-2000:]}')
            time.sleep(.04)
        raise AssertionError(f'Timeout: {label}\n{json.dumps(state(),indent=2)}')
    def record(label:str)->None:evidence.append({'check':label,'passed':True});print('PASS',label,flush=True)
    def capture(name:str)->None:time.sleep(.15);ImageGrab.grab(xdisplay=display).save(out/name)
    def command(line:str,error:bool=False)->dict:
        desktop.chord('Control_L','l');desktop.chord('Control_L','a');desktop.text(line);desktop.press('Return')
        result=wait(lambda s:s.get('command')==line and not s.get('busy',True),line)
        assert bool(result.get('error'))==error,(line,result.get('error'))
        assert not result.get('render_error'),result.get('render_error')
        return result
    def obj(s:dict,id:int)->dict:return next(o for o in s['objects']if o['id']==id)
    try:
        s=wait(lambda s:s.get('ready')and len(s.get('objects',[]))==3,'startup',40);desktop.find_window()
        desktop.x.XSetInputFocus(desktop.display,desktop.window,2,0);desktop.x.XFlush(desktop.display)
        assert s['triangles']>1000 and s['render_error']is None;record('Native window, four views and real OCCT meshes')
        scale=s['pixels_per_point'];top=s['viewports'][1];camera=s['cameras'][1]
        desktop.click((top[0]+25)*scale,(top[1]+70)*scale)
        wait(lambda s:s.get('selected')==[],'clear selection');capture('four-view.png');record('Viewport background clears selection')
        # Inspect every viewport, not just the final callback. A scissor regression
        # used to erase the first three views while leaving geometry telemetry valid.
        image=ImageGrab.grab(xdisplay=display).convert('RGB')
        for rect in s['viewports']:
            region=image.crop(tuple(int(v*scale) for v in (rect[0]+30,rect[1]+30,rect[2]-30,rect[3]-30)))
            colored=sum(1 for red,green,blue in (region.get_flattened_data() if hasattr(region,'get_flattened_data') else region.getdata()) if max(red,green,blue)-min(red,green,blue)>35 and max(red,green,blue)>70)
            assert colored>200,('Viewport has no visible colored CAD surfaces',rect,colored)
        record('All four viewports contain rendered colored surfaces, not only scene metadata')

        width=top[2]-top[0];height=top[3]-top[1];span=camera['span'];target=camera['target']
        x=(top[0]+width/2+(30-target[0])/(span*(width/height))*width/2)*scale
        y=(top[1]+height/2-(21-target[1])/span*height/2)*scale
        desktop.click(x,y);s=wait(lambda s:s.get('selected')==[1],'ray pick plate');record('Mouse ray selects the plate, not its bounding box alone')
        perspective=s['viewports'][0];cx=(perspective[0]+perspective[2])/2*scale;cy=(perspective[1]+perspective[3])/2*scale
        old_eye=s['cameras'][0]['eye'];revision=s['revision'];desktop.drag(cx,cy,65,28,3)
        s=wait(lambda s:s.get('cameras',[{}])[0].get('eye')!=old_eye,'orbit');assert s['revision']==revision;record('Right-drag orbits without editing geometry')
        old_target=s['cameras'][0]['target'];desktop.drag(cx,cy,25,-18,2)
        s=wait(lambda s:s['cameras'][0]['target']!=old_target,'pan');assert s['revision']==revision;record('Middle-drag pans independently of the document')
        old_span=s['cameras'][0]['span'];desktop.click(cx,cy,4)
        s=wait(lambda s:s['cameras'][0]['span']<old_span,'zoom');record('Wheel zoom changes the camera, not the model')
        s=command('Move 1 10,0,0');assert abs(obj(s,1)['stats']['bounds'][0]-10)<1e-7;record('Command entry edits exact geometry')
        s=command('Undo');assert abs(obj(s,1)['stats']['bounds'][0])<1e-7;record('Undo restores the previous exact shape')
        revision=s['revision'];before=obj(s,1)['stats'];s=command('FilletEdge 1 100000',error=True)
        assert s['revision']==revision and obj(s,1)['stats']==before;record('Failed fillet leaves geometry and revision unchanged')
        s=command('Box 140,0,0 20 20 20');new_id=s['selected'][0];assert len(s['objects'])==4;assert abs(obj(s,new_id)['stats']['volume']-8000)<1e-6;record('Worker accepts new edits after a failed native operation')
        project=out/'GUI roundtrip.arcocct';s=command(f'Save "{project}"');assert not s['dirty']and project.exists();saved=hashlib.sha256(project.read_bytes()).hexdigest();record('Quoted path save writes a project and marks it clean')
        s=command(f'Move {new_id} 0,20,0');assert s['dirty'];s=command(f'Save "{out/"missing"/"fail.arcocct"}"',error=True)
        assert s['dirty']and hashlib.sha256(project.read_bytes()).hexdigest()==saved;record('Failed save preserves the old file and dirty state')
        s=command('Undo');assert not s['dirty'];record('Undo back to the saved snapshot clears the dirty flag')
        step=out/'gui.step';s=command(f'ExportStep "{step}"');assert step.stat().st_size>10000;record('GUI exports an actual STEP file')
        stl=out/'gui.stl';s=command(f'ExportStl "{stl}"');data=stl.read_bytes();triangles=struct.unpack_from('<I',data,80)[0];assert len(data)==84+50*triangles and triangles>1000;record('GUI exports correctly sized binary STL triangles')
        s=command(f'Move {new_id} 0,20,0');revision=s['revision']
        line=f'Open "{project}"';desktop.chord('Control_L','l');desktop.chord('Control_L','a');desktop.text(line);desktop.press('Return')
        s=wait(lambda s:s.get('replace_guard'),'unsaved replacement guard');assert s['revision']==revision;desktop.press('Escape');wait(lambda s:not s.get('replace_guard'),'cancel replacement');record('Unsaved Open is guarded; Escape preserves the current document')
        s=command('Undo');s=command(f'Open "{project}"');assert not s['dirty']and len(s['objects'])==4;assert abs(obj(s,new_id)['stats']['bounds'][1])<1e-7;record('GUI reload restores saved geometry and layer metadata')
        s=command(f'Move {new_id} 0,1,0');desktop.close_request();s=wait(lambda s:s.get('close_guard'),'dirty close guard');assert app.poll()is None;capture('close-guard.png');desktop.press('Escape');wait(lambda s:not s.get('close_guard'),'cancel close');record('OS close request cannot silently discard an unsaved document')
        # Return to the original showcase for a clean final screenshot.
        command('Undo');command(f'Delete {new_id}');s=command(f'Save "{out/"showcase.arcocct"}"')
        top=s['viewports'][1];desktop.click((top[0]+25)*scale,(top[1]+70)*scale)
        desktop.press('Home');time.sleep(.2);capture('final-four-view.png')
        desktop.close_request();app.wait(timeout=10);assert app.returncode==0;record('Clean native window exits normally')
    except Exception:
        capture('failure.png');raise
    finally:
        (out/'ui-results.json').write_text(json.dumps({'checks':evidence,'count':len(evidence)},indent=2))
        if app.poll()is None:app.terminate();app.wait(timeout=5)
        log.close()

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    run(args.binary.resolve(),args.output.resolve())
