# keynav-wayland
keynav-wayland is an implementation of
[keynav](https://www.semicomplete.com/projects/keynav/) for wayland! Right now
it is not directly compatible with existing keynavrc, but compatibility and more
documentation should be coming soon!

It works, and is already useful for me at least, but still has some work to be
done. Please use it and let me know what you think.

*Note:* By design this app will take over your entire screen and capture
virtually all keyboard input. The default configuration uses escape to exit the
app, but if for some reason that does not work (ie. you broke it while
developing) you can switch to another terminal via control+F<2,3,3,..> and kill
keynav-wayland from there. While developing it can be useful to run this app
with a timeout. Making it harder to for this app to lock you out is in the
Todos.

## Usage
Typical usage involves seting up a keybinding to start this app eg with

```
bindsym Control+semicolon exec keynav-wayland
```

in sway. The app will then present you with a rectangle enclosed cross hairs,
which in the default configuration you can narrow with h, j, k, l, move with H,
J, K, L, click with enter, and exit with escape.

## Compatability
Aside from core wayland, this app requires the [wlr layer
shell](https://wayland.app/protocols/wlr-layer-shell-unstable-v1) and [virtual
pointer](https://wayland.app/protocols/wlr-virtual-pointer-unstable-v1)
protocols, both of which are unstable. This app should work on any wlroots based
compositor but has only been tested on sway.

## TODO
- [ ] improve readme
- [ ] Make it harder to lock yourself out (eg. make unrecognized keys quit)
- [ ] multi monitor support
- [ ] Consult the original to see what I'm missing, consider whether going
      config compatible is worth it
- [ ] Add more verbs
- [ ] Clean up code
- [ ] Document configuration language
- [ ] Spread word
- [ ] Package
