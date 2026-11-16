from pathlib import Path
import tarfile
root=Path(__file__).resolve().parents[1]
with tarfile.open(root/'output/orange-ring-vendor.tar.gz','w:gz') as t:t.add(root/'private/vendor/ring',arcname='vendor/ring')
print('ring vendor archived')
