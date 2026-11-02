//! Layers: the design saves `layers` (the listed layers with their names and colours, the
//! current layer, and which layer each body is on). A design without them loads as before:
//! every body is on the default layer, and the field is left out when nothing is set. The last
//! release does not read this field, so it refuses a design that has it (it refuses the newer
//! format number), rather than dropping the layers.
