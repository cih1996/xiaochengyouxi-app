"""Snapshot server sources for an isolated target-platform test run."""
from pathlib import Path
import tarfile
root=Path(__file__).resolve().parents[1]
with tarfile.open(root/'output/orange-server-check.tar.gz','w:gz') as archive:
    for name in ['server/Cargo.toml','server/Cargo.lock','server/src']:
        archive.add(root/name,arcname=name)
print('Server check snapshot prepared.')
