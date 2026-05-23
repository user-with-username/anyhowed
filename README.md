# anyhowed

Its anyhow. But the word "error:" is red

## What

Exactly anyhow. Error chaining. Context. Downcast. bail. ensure. All of it

The only difference? The `"error:"` word in your terminal shows up red

## Example

```rust
use anyhowed::{Result, anyhow, bail, Context};

fn risky() -> Result<()> {
    bail!("something broke")
}

fn main() -> Result<()> {
    risky().context("it failed")?;
    Ok(())
}
```

## Credits

This crate is a lightweight, colored clone of the amazing [anyhow](https://crates.io/crates/anyhow) crate by David Tolnay

License

[MIT](LICENSE)
