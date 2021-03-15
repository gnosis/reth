## Design notes

 - Let's try to have as few top-level crates as possible. A top-level crate is an entire area of ownership that could potentially be its own team.
 - `core` should have no dependencies on other crates at all.
