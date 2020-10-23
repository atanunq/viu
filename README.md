# viu

### Description
A small command-line application to view images from the terminal written in Rust. It is basically the
front-end of [`viuer`](https://github.com/atanunq/viuer). It uses the
[Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol.html), if supported.
If not, lower half blocks (▄ or \u2584) are displayed insread.

Based on the value of `$TERM`, `viuer` decides whether to use the Kitty protocol or not. For half
blocks, `$COLORTERM` is inspected. If it contains either `truecolor` or `24bit`,
truecolor (16 million colors) will be used. If not, it will fallback to using only ansi256. A nice
explanation can be found in this [gist](https://gist.github.com/XVilka/8346728).


Features (see [Usage](#usage)):
- Animated GIF support
- Accept media through stdin
- Custom dimensions
- Transparency

### Installation

#### From source (recommended)

##### Standard
Installation from source requires a local [Rust
environment](https://www.rust-lang.org/tools/install). Please note that at least Rust 1.39 is required.

```bash
git clone https://github.com/atanunq/viu.git

# Build & Install
cd viu/
cargo install --path .

# Use
viu img/giphy.gif
```
Or without cloning:
```bash
cargo install viu
```

##### [WASI](https://github.com/wasmerio/wasmer)
First, you will need the WASI target installed in your Rust system:

```bash
rustup target add wasm32-wasi --toolchain nightly
```

Once WASI is available, you can build the WebAssembly binary by yourself with:

```bash
cargo +nightly build --release --target wasm32-wasi
```

This will create a new file located at `target/wasm32-wasi/release/viu.wasm`.

When the wasm file is created you can upload it to wapm or execute it with wasmer:

```bash
wapm publish
# OR
wasmer run  target/wasm32-wasi/release/viu.wasm --dir=. -- img/giphy.gif
```

#### Binary
A precompiled binary can be downloaded from the [release
page](https://www.github.com/atanunq/viu/releases/latest).

#### From wapm

Viu can be installed in Linux, macOS and Windows using [wapm](https://wapm.io/):

```bash
wapm install -g viu
```

#### Packages

##### Arch Linux
There is an [AUR package available for Arch Linux](https://aur.archlinux.org/packages/viu/).

### Usage
On a [Kitty](https://github.com/kovidgoyal/kitty) terminal:

![Kitty](img/kittydemo.gif)


Using half blocks (Kitty protocol and `tmux` do not get along):

![Demo](img/demo.gif)


![Demo](img/gifdemo.gif)


![Demo](img/curldemo.gif)


Ctrl-C was pressed to stop the GIFs.


Examples:

- `viu img/giphy.gif`
- `viu img/*`
- `viu ~/Pictures -rn`


The shell will expand the wildcard above and `viu` will display all the images in the folder one
after the other. For a more informative output when dealing with folders the flag **-n** could be
used.

When `viu` receives only one file and it is GIF, it will be displayed over and over until Ctrl-C is
pressed. However, when couple of files are up for display (second example) the GIF will be displayed
only once.

##### Aspect Ratio
If no flags are supplied to `viu` it will try to get the size of the terminal where it was invoked.
If it succeeds it will fit the image and preserve the aspect ratio. The aspect ratio will be changed
only if both options **-w** and **-h** are used together.

##### Command line options
```
USAGE:
    viu [FLAGS] [OPTIONS] [FILE]...
    When FILE is -, read standard input.

FLAGS:
    -m, --mirror         Display a mirror of the original image
    -n, --name           Output the name of the file before displaying
    -1, --once           Only loop once through the animation
    -r, --recursive      Recurse down directories if passed one
    -s, --static         Show only first frame of gif
    -t, --transparent    Display transparent image with transparent background
    -v, --verbose        Output what is going on

OPTIONS:
    -h, --height <height>    Resize the image to a provided height
    -w, --width <width>      Resize the image to a provided width

ARGS:
    <FILE>...    The image to be displayed
```
