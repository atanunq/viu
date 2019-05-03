# viu

### Description
A small command-line application to view images from the terminal written in Rust. 
It uses lower half blocks (▄ or \u2584) to fit 2 pixels into a single cell by adjusting foreground and background colours accordingly.

### Installation

#### From source
Installation from source required a local [Rust environment](https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/atanunq/viu.git

# Build & Install
cd viu/
cargo install --path .

# Use
viu img/smallimage.jpg
```
#### Binary
A precompiled binary can be downloaded from the [release page](https://www.github.com/atanunq/viu/releases/latest).

#### Packages

##### Arch Linux
There is an [AUR package available for Arch Linux](https://aur.archlinux.org/packages/viu/).

### Usage

![Demo](img/demo.gif)

Examples:

- `viu img/smallimage.jpg` 
- `viu img/*`


The shell will expand the wildcard above and *viu* will display all the images in the folder one after the other. For a more informative output when dealing with folders the flag **-n** could be used.

##### Aspect Ratio
If no flags are supplied to *viu* it will try to get the size of the terminal where it was invoked. If it succeeds it will fit the image and preserve the aspect ratio. The aspect ratio will be changed only if both options **-w** and **-h** are used together.

##### Command line options
```
USAGE:
    viu [FLAGS] [OPTIONS] <FILE>...

FLAGS:
    -m, --mirror     Display a mirror of the original image
    -n, --name       Output the name of the file before displaying
    -v, --verbose    Output what is going on

OPTIONS:
    -h, --height <height>    Resize the image to a provided height
    -w, --width <width>      Resize the image to a provided width

ARGS:
    <FILE>...    The image to be displayed
```
