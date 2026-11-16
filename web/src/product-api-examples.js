export function productApiExamples(baseUrl, separateImage = false) {
  const starts = new Date(Math.floor(Date.now() / 1000) * 1000);
  const payload = {
    title: "日用精选纸巾",
    url: "https://example.com/product/10001",
    price: "29.90",
    commission_percent: "20",
    ...(separateImage
      ? { image_id: "上传图片接口返回的 image_id" }
      : { image_base64: "图片文件的 Base64 字符串" }),
    external_id: "shop-10001",
    starts_at: starts.toISOString(),
    expires_at: new Date(starts.getTime() + 7 * 86400000).toISOString(),
    enabled: true,
  };
  const imageUpload = `image_request = urllib.request.Request(
    BASE_URL + "/api/integration/product-images",
    data=json.dumps({"image_base64": image_base64}).encode("utf-8"),
    headers=headers, method="POST")
with urllib.request.urlopen(image_request) as response:
    image_id = json.load(response)["image_id"]
`;
  const python = `import base64
import json
import os
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

BASE_URL = ${JSON.stringify(baseUrl)}
headers = {
    "Content-Type": "application/json",
    "X-Product-Api-Key": os.environ["PRODUCT_API_KEY"],
}
image_base64 = base64.b64encode(Path("product.jpg").read_bytes()).decode("ascii")
${separateImage ? imageUpload : ""}
starts_at = datetime.now(timezone.utc).replace(microsecond=0)
payload = {
    "title": "日用精选纸巾",
    "url": "https://example.com/product/10001",
    "price": "29.90",
    "commission_percent": "20",
    "${separateImage ? "image_id" : "image_base64"}": ${separateImage ? "image_id" : "image_base64"},
    "external_id": "shop-10001",
    "starts_at": starts_at.isoformat(),
    "expires_at": (starts_at + timedelta(days=7)).isoformat(),
    "enabled": True,
}
request = urllib.request.Request(
    BASE_URL + "/api/integration/products",
    data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
    headers=headers, method="POST")
with urllib.request.urlopen(request) as response:
    print(response.read().decode("utf-8"))`;
  return { json: JSON.stringify(payload, null, 2), python };
}
