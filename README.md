# Matrix digital rain

![](demo.gif)
This emulates the classic Matrix digital rain effect on your terminal. I mostly wrote this to learn Rust.

If you run it, the only way to quit currently is hitting `q`. Need to learn some more Rust to make it more ergonomic :)

## Parameters

There are some parameters for the rendering that you can control with some environment variables:

- `TRAIL_DENSITY`: the program renders 1 rain "trail" per `TRAIL_DENSITY` terminal squares. By default this is set to 30.
- `RAIN_CHARSET`: the characters in the string set for this variable will be sampled for the characters in the digital rain. There are some Japanese and English characters by default, picked arbitrarily to resemble the original from the movie.