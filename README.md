# fileforge

A Rust CLI that organizes a messy folder by moving files into per-extension
subfolders. Has a --dry-run flag so you can preview before committing.
My first serious Rust project: clap for args, std::fs for the heavy lifting.

## Behaviour

- A file whose destination name is already taken is left alone and reported
  as `skipped ... (already exists)`. Nothing is ever overwritten.
- A move that fails does not end the run: fileforge names the file, says what
  it was trying to do, and carries on with the rest. An entry it cannot even
  inspect is reported the same way rather than dropped.
- A run that can read the directory ends with a summary line,
  `organized N files, skipped M`, and exits 1 if any move failed. Name
  collisions alone are not a failure. If the directory itself cannot be read,
  fileforge says so and exits 1 without a summary.

```sh
$ fileforge ~/Downloads
moved /home/me/Downloads/a.pdf -> pdf/
skipped /home/me/Downloads/b.pdf -> pdf/: cannot move to /home/me/Downloads/pdf/b.pdf: Permission denied (os error 13)
organized 1 file, skipped 1
```
