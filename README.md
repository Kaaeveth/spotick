# Spotick
A simple and lightweight Windows [widget](https://en.wikipedia.org/wiki/Software_widget) for media applications like Spotify, your default music player or just your browser.
Unlike the built-in Windows media UI, Spotick allows selecting one application to display media information for and doesn't fade away.
In addition, Spotick may be placed anywhere on your Desktop, resized to your liking and can stay on top of other windows.
Any application using the Windows Media Control is supported.

![Spotick](https://github.com/user-attachments/assets/4910fcf7-e9e4-44c1-a208-05fb94cc1561)
![Spotick-Spotify](https://github.com/user-attachments/assets/9043d20a-8435-4cd8-bb21-6c3f33032dca)

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
