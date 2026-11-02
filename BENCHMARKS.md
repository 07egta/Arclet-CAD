# Arclet CAD measurements (2026-11-02)

These are the only numbers measured so far. They are rough and not a performance claim.

| What | Result | Conditions |
|---|---|---|
| Resident memory, idle at the empty start screen (debug build) | about 298 MB peak | Linux container, virtual X display, software OpenGL (no GPU), 1280x800, 8 s after launch |
| Resident memory, idle (release build from the previous day, not rebuilt from the latest commit) | about 270 MB peak | same |
| Idle CPU | about 2.4 % averaged over the process lifetime | same |
| Threads | 9 | same |

Not measured, so nothing is claimed: start-up time (the probe I wrote did not detect the window, so its 21 s value is discarded), frame rate, input latency, control-point drag, Rebuild/Loft/Boolean timing, STEP/3DM load time, large models, GPU use, Windows and macOS. No comparison with any other CAD program has been made.

Method: `Xvfb` + `WGPU_BACKEND=gl`, read `VmRSS`/`VmHWM` from `/proc/<pid>/status`. Software rendering makes CPU and timing numbers unrepresentative of real hardware.
