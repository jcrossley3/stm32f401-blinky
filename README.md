# stm32f401-blinky

Plug in your board and run this:

    $ cargo install cargo-flash
    $ cargo flash --chip stm32f401re

The commits in this repo document my efforts to get my
[NUCLEO-F401RE](https://www.digikey.com/product-detail/en/stmicroelectronics/NUCLEO-F401RE/497-14360-ND/4695525)
running something somewhat useful.

I found a fairly old (in embedded-rust years) [blinky tutorial for a
slightly different
card](https://beta7.io/posts/embedded-rust-from-zero-to-blinky.html)
and decided to see if I could adapt it to mine. In the course of doing
so, I discovered a better approach would've been to use a blinky
example from one of the crates for my particular card, specifically
the
[stm32f4xx-hal](https://github.com/stm32-rs/stm32f4xx-hal/blob/master/examples/delay-blinky.rs)
and
[nucleo-f401re](https://github.com/jkristell/nucleo-f401re/blob/master/examples/timer-blinky.rs)
crates. Comparing those to the one in the tutorial is a useful
exercise, since they each take slightly different approaches to
blinking an LED.

Since I'm stubborn, I wanted to see if I could make the code from the
old tutorial work. With the above examples and the rust compiler,
fixing the code was simple enough, but because I'm running Fedora 31,
I couldn't make the gdb/openocd flashing work due to [this
bug](https://github.com/rust-embedded/book/issues/249), so I used the
more modern [cargo-flash](https://crates.io/crates/cargo-flash) crate
from the [probe.rs](https://probe.rs/) project instead. That worked.

