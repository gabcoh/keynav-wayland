# keynav-wayland
keynav-wayland is an implementation of
[keynav](https://www.semicomplete.com/projects/keynav/) for wayland!
It may not be drop in compatible with your existing keynavrc, but it supports
most of the core keynav commands and just requires a few slight modifications to
how keybindings are recorded (comprehensive documentation to come).

## Usage
Start by creating a config at `$XDG_CONFIG_HOME/keynav/keynavrc`. A good starting point is:
```
Escape end
Control+bracketleft end
h cut-left
j cut-down
k cut-up
l cut-right
Shift+h move-left
Shift+j move-down
Shift+k move-up
Shift+l move-right
space warp,click 1,end
Return warp,click 1,end
semicolon warp,end
w warp
c cursorzoom 300 300
e end
1 click 1
2 click 2
3 click 3
Control+h cut-left
Control+j cut-down
Control+k cut-up
Control+l cut-right
y cut-left,cut-up
u cut-right,cut-up
b cut-left,cut-down
n cut-right,cut-down
Shift+y move-left,move-up
Shift+u move-right,move-up
Shift+b move-left,move-down
Shift+n move-right,move-down
Control+y cut-left,cut-up
Control+u cut-right,cut-up
Control+b cut-left,cut-down
Control+n cut-right,cut-down
```

Typically you'll then want to set up a keybinding to start this app eg with

```
bindsym Control+semicolon exec keynav-wayland
```

in sway.

## Build/Install
No tricks here, just standard `cargo build` and/or `cargo install`.

## Compositor Compatability
Aside from core wayland, this app requires the [wlr layer
shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) and [virtual
pointer](https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1)
protocols, both of which are unstable. This app should work on any wlroots based
compositor but has only been tested on sway.

## TODO
- [ ] multi monitor support
- [ ] Add remaining relevant verbs from keynav (eg. macros and history)
- [ ] Clean up code (!!!)
- [ ] Document
- [ ] Spread word
- [ ] Package
