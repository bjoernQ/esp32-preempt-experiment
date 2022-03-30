# Experiment for preemtive multitasking on ESP32 in bare-metal Rust

This is just a small experiment having a main task and two additional tasks running on ESP32.

The code is meant to be as simple as possible to make it easy to understand.

Please note: Since this uses simple `for`-loops as delays it won't work in release mode!
