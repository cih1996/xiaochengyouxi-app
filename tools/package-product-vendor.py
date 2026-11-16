"""Archive only newly required multipart dependencies for the offline server."""
from pathlib import Path
import tarfile

root=Path(__file__).resolve().parents[1]
vendor=root/'private/vendor'
names=['multer','encoding_rs','spin','scopeguard','simdutf8','core_detect','multiversion_no_op']
with tarfile.open(root/'output/orange-product-vendor.tar.gz','w:gz') as archive:
    for name in names:
        source=vendor/name
        if not (source/'.cargo-checksum.json').exists():
            raise RuntimeError(f'Missing vendored crate: {name}')
        archive.add(source,arcname='vendor/'+name)
print('Multipart dependency archive prepared.')
