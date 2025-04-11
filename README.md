# Windows 9x EFI
This project is an EFI bootloader for Windows 9x (incl. Me).

The idea is simple: load the kernel and registry into memory, and boot it.

Build: `cargo build --target i686-unknown-uefi`
Run: Copy `target/i686-unknown-uefi/debug/windows-9x-efi.efi` to `C:\EFI\Boot\bootia32.efi` on the target.

- [x] Load kernel and registry from FAT32
- [x] Decompress kernel, if necessary
- [ ] Actually boot the kernel
