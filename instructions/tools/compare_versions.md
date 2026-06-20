Compare two exact crate versions. Returns public rustdoc items added and removed, items whose structured signatures changed, feature-definition changes, manifest dependency changes, and MSRV changes.

Use this as the first step for upgrades and migration questions. Follow up with `get_item` for changed APIs and `changelog` for author-written release notes. Both versions must have compatible rustdoc JSON; target resolution is reported independently for each version.
