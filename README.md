# Spotick
A simple and lightweight Windows [widget](https://en.wikipedia.org/wiki/Software_widget) for media applications like Spotify.
Unlike the built-in Windows media UI, Spotick allows to select one application to display media information for.
Any application using the Windows Media Control is supported.

![Spotick](https://github.com/user-attachments/assets/0792d980-b1f6-470a-83e4-b4312510a367)

## Development
Simply use `cargo` to compile and run the project.
The [Slint.slint](https://marketplace.visualstudio.com/items?itemName=Slint.slint) extension for VSCode allows to preview
any changes made to the UI and an LSP for `.slint` files.
Use [slint-viewer](https://github.com/slint-ui/slint/tree/master/tools/viewer) if you don't use VSCode.

## TODO
* [x] Persist widget position
* [x] Autostart
* [x] Make widget resizable/scalable
* [ ] Display track position (seek position?)
* [ ] Display and change volume
* [ ] Keybindings
