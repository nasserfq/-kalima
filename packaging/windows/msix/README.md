# كلمة in the Microsoft Store

`make-msix.ps1` turns a release build into `kalima-<version>-windows-x64-store.msix`. The release
workflow builds it on every push to `release` and keeps it as the **microsoft-store** workflow
artifact; it is not attached to the GitHub release (the website downloads stay as they are).

The package identity in `AppxManifest.xml.in` is the one Partner Center reserved for كلمة
(Product management › Product identity). The Store signs the package when it is uploaded.

## What changes in a Store copy

A copy installed from the Store (it runs from a `WindowsApps` folder, `craft_update::store`):

- never checks for or downloads updates; Help › Check for Updates opens the Store's updates page;
- never runs the Arabic spell-checker install that asks to run as administrator. If Windows has no
  Arabic spell checker, add Arabic under Settings › Time & language › Language.

The same `Kalima.exe` is in the website zip, which keeps its own updater.

## Trying the package before uploading

On a Windows PC with the Windows SDK, as administrator:

```powershell
$cert = New-SelfSignedCertificate -Type Custom -Subject "CN=AF2BB125-FF7A-4337-932D-42917A905572" `
  -KeyUsage DigitalSignature -CertStoreLocation Cert:\CurrentUser\My `
  -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
Export-Certificate -Cert $cert -FilePath test.cer
Import-Certificate -FilePath test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople
Copy-Item kalima-*-store.msix test.msix
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe" sign /fd SHA256 /sha1 $cert.Thumbprint test.msix
Add-AppxPackage test.msix
```

Then check: open a .docx from Explorer, save, print, `kalima-cli --help` in a new terminal, and
Arabic spelling. Remove it with `Get-AppxPackage *22450D6D75B7D* | Remove-AppxPackage`.

## Uploading

Partner Center › كلمة › Start submission › Packages: upload the unsigned `.msix` from the workflow
artifact. Each new upload needs a higher version, which the workflow's run number gives.
