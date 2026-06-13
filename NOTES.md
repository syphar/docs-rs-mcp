# notes

## TODO:

- prefetch rustdoc json when we get resolve-version requests, including external
  dependencies?
- on-disk sqlite representation for faster queries?
- try to locate .crate file in cargo cache? or even `/src/` cargo cache?
- don't do a rustdoc json request when rustdoc_status = false? for a better
  error message?
