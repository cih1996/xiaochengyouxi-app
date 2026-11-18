"""COS helper for release files. Credentials stay in private/cos.json; every key is forced under
the project prefix (the bucket is shared with other business).

  python tools/cos.py put <local> <key>     # key relative to the prefix, e.g. releases/x.apk
  python tools/cos.py head <key>
  python tools/cos.py url <key> [seconds]   # presigned GET URL
  python tools/cos.py ls [sub-prefix]
  python tools/cos.py rm <key>
"""
import hashlib
import json
import sys
import time
from pathlib import Path

from qcloud_cos import CosConfig, CosS3Client

root = Path(__file__).resolve().parents[1]
cfg = json.loads((root / "private/cos.json").read_text(encoding="utf-8"))
prefix = cfg["prefix"].strip("/") + "/"
client = CosS3Client(CosConfig(Region=cfg["region"], SecretId=cfg["secret_id"], SecretKey=cfg["secret_key"], Scheme="https"))
bucket = cfg["bucket"]


def full(key: str) -> str:
    key = key.lstrip("/")
    if ".." in key.split("/"):
        raise SystemExit("invalid key")
    return key if key.startswith(prefix) else prefix + key


def main(argv):
    action, *rest = argv
    if action == "put":
        local, key = rest
        data = Path(local).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        start = time.time()
        client.upload_file(Bucket=bucket, LocalFilePath=local, Key=full(key), PartSize=4, MAXThread=4,
                           Metadata={"x-cos-meta-sha256": digest})
        took = time.time() - start
        print(f"uploaded {full(key)} {len(data)} bytes in {took:.1f}s ({len(data) / 1024 / max(took, 0.01):.0f} KB/s) sha256={digest}")
    elif action == "head":
        head = client.head_object(Bucket=bucket, Key=full(rest[0]))
        print(json.dumps({k: v for k, v in head.items() if k.lower() in ("content-length", "etag", "last-modified", "x-cos-meta-sha256")}, ensure_ascii=False))
    elif action == "url":
        seconds = int(rest[1]) if len(rest) > 1 else 3600
        print(client.get_presigned_url(Method="GET", Bucket=bucket, Key=full(rest[0]), Expired=seconds))
    elif action == "ls":
        sub = full(rest[0]) if rest else prefix
        resp = client.list_objects(Bucket=bucket, Prefix=sub, MaxKeys=200)
        for item in resp.get("Contents", []):
            print(item["Size"], item["LastModified"], item["Key"])
    elif action == "rm":
        client.delete_object(Bucket=bucket, Key=full(rest[0]))
        print("deleted", full(rest[0]))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv[1:])
