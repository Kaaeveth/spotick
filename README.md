# Spotick
A simple and lightweight Windows widget for media applications like Spotify.
Unlike the built-in Windows media UI, Spotick allows to select one application to display media information for.
Any application using the Windows Media Control is supported.

## Development
Simply use `cargo` to compile and run the project.
The [Slint.slint](https://marketplace.visualstudio.com/items?itemName=Slint.slint) extension for VSCode allows to preview
any changes made to the UI and an LSP for `.slint` files.
Use [slint-viewer](https://github.com/slint-ui/slint/tree/master/tools/viewer) if you don't use VSCode.

## TODO
* [x] Persist widget position
* [x] Autostart
* [ ] Make widget resizable/scalable
* [ ] Display track position (seek position?)
* [ ] Display and change volume
* [ ] Keybindings
