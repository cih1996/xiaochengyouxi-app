import { rm } from "node:fs/promises";
const target = new URL("../public/docs/PRODUCT-API.md", import.meta.url);
// 商品接入文档仅在后台会话中展示，不能作为公开静态文件发布。
await rm(target, { force: true });
