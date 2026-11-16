"""Package the matching Rust server and built admin/site assets."""
from pathlib import Path
import tarfile

root=Path(__file__).resolve().parents[1]
with tarfile.open(root/'output/orange-admin-update.tar.gz','w:gz') as archive:
    for name in ['server/Cargo.toml','server/Cargo.lock','server/src','web/dist']:
        archive.add(root/name,arcname=name)
print('Server and website update archive prepared.')
