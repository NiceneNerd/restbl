# restbl

A simple library to handle RSTB/RESTBL (resource size table) files from *The
Legend of Zelda: Tears of the Kingdom*. Features:
- Quick, zero-allocation parser
- Optional `alloc` feature to support editable table which can be serialized to
  binary or (with the `yaml` feature) YAML.
- `no_std` support (optional `std` feature)
- optional Serde support (`serde` feature)
- `aarch64-nintendo-switch-freestanding` support (without the `std` feature)

## Example Usage

```rust
use restbl::bin::ResTblReader;

let bytes = std::fs::read("test/ResourceSizeTable.Product.110.rsizetable").unwrap();

// Setup the quick, zero-allocation reader
let reader = ResTblReader::new(bytes.as_slice()).unwrap();
// Lookup an RSTB value
assert_eq!(
    reader.get("Bake/Scene/MainField_G_26_43.bkres"),
    Some(31880)
);

#[cfg(feature = "alloc")]
{
    use restbl::ResourceSizeTable;
    // Parse RSTB into owned table
    let mut table = ResourceSizeTable::from_parser(&reader);
    // Set the size for a resource
    table.set("TexToGo/Etc_BaseCampWallWood_A_Alb.txtg", 777);
    // Check the size
    assert_eq!(
        table.get("TexToGo/Etc_BaseCampWallWood_A_Alb.txtg"),
        Some(777)
    );
    // Dump to YAML, if `yaml` feature enabled
    #[cfg(feature = "yaml")]
    {
        let json_table = table.to_text();
        // From YAML back to RSTB
        let new_table = ResourceSizeTable::from_text(&json_table).unwrap();
    }
}
```

## Building for Switch

To build for Switch, you will need to use the
`aarch64-nintendo-switch-freestanding` target. The `std` feature is not
supported, so you will need to use `--no-default-features`. Since [`cargo
nx`](https://github.com/aarch64-switch-rs/cargo-nx) does not seem to support
passing feature flags, you will need to run the full command yourself, as
follows:

```
cargo build -Z build-std=core,compiler_builtins,alloc --target aarch64-nintendo-switch-freestanding --no-default-features
```

## License

This software is licensed under the terms of the GNU General Public License,
version 3 or later.

License: GPL-3.0-or-later
