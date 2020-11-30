# stm32f401-esp8266

This should attempt to post some data to your drogue-cloud
http-endpoint running on your minikube cluster.

First, set the const values at the top of [src/main.rs](src/main.rs) appropriately
for your local network.

Then plug in your board and run this:

    $ cargo install cargo-embed
    $ cargo embed

