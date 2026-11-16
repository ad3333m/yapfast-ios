# iPhone (IPA)

The `ios` branch builds Yapfast for iPhone and iPad, `aarch64-apple-ios`, as an
unsigned IPA to sideload: `.github/workflows/ios.yml` on an Intel Mac runner
(the Apple Silicon queue can take hours), `packaging/ios` for the bundle's
`Info.plist` and icons.

What the desktop build has and iOS doesn't:

- File dialogs: rfd has no iOS backend, so `src/ios_rfd.rs` stands in for it
  and every dialog picks nothing.
- Pictures on the clipboard: arboard has no iOS backend; copying a picture
  says so in the log and pasting one does nothing.
