# B站音视频流 CDN 分类

> 参考: https://github.com/the1812/Bilibili-Evolved/issues/3234#issuecomment-1504764774

## CDN 类型总览

| 类型 | 域名格式 | os 参数 | 质量 | 特点 |
|---|---|---|---|---|
| Mirror | `upos-(sz\|hz\|bstar)-mirror*.bilivideo.com` | `ali`/`cos`/`hw` 等 | 最高 | 依托大厂 CDN，稳定快速 |
| UPOS | `upos-sz-estg*.bilivideo.com` | `upos` | 中等 | 直接取对象存储，冷门视频 |
| BCache | `cn-*.bilivideo.(com\|cn)` | `bcache` | 因地区而异 | 自建机房 |
| MCDN | 包含 `mcdn` | `mcdn` | 最差 | PCDN，不稳定 |
| IP:Port | `http://IP:Port/v1/resource/*` | — | 最差 | PCDN，APP 端未开 HTTPS |
| 免流 | `(upos\|proxy).*-tf-*.bilivideo.com` | — | — | 不认 upsig |
| 迅雷 | `*.szbdyd.com` | — | — | 已停用 |

---

## Mirror 型 CDN

域名 Regex: `^upos-(sz|hz|bstar)-mirror([0-9,a-z]+)\.(bilivideo\.com|akamaized\.net)$`

判断方式: URL 参数 `os=ali` 等（和 `mirror` 后面到 `.` 之间的字符串一致）

特点: 最稳定，速度最快。部分有 UA/Refer 限制，部分非全地域可用。

### 国内 Mirror

| 域名 | 厂商 | 备注 |
|---|---|---|
| `upos-sz-mirrorali.bilivideo.com` | 阿里云 | |
| `upos-sz-mirroralib.bilivideo.com` | 阿里云 | |
| `upos-sz-mirroralio1.bilivideo.com` | 阿里云 | |
| `upos-sz-mirrorbd.bilivideo.com` | 百度云 | |
| `upos-sz-mirrorcos.bilivideo.com` | 腾讯云 | |
| `upos-sz-mirrorcosb.bilivideo.com` | 腾讯云 | VOD 加速 |
| `upos-sz-mirrorcoso1.bilivideo.com` | 腾讯云 | |
| `upos-sz-mirrorhw.bilivideo.com` | 华为云 | 融合 CDN |
| `upos-sz-mirrorhwb.bilivideo.com` | 华为云 | 融合 CDN |
| `upos-sz-mirrorhwo1.bilivideo.com` | 华为云 | 融合 CDN |
| `upos-sz-mirror08c.bilivideo.com` | 华为云 | 融合 CDN |
| `upos-sz-mirror08h.bilivideo.com` | 华为云 | 融合 CDN |
| `upos-sz-mirror08ct.bilivideo.com` | 华为云 | 融合 CDN |

### 海外 Mirror

| 域名 | 厂商 | 区域 | 备注 |
|---|---|---|---|
| `upos-hz-mirrorakam.akamaized.net` | Akamai | 海外 | 有参数校验，其他类型不能直接替换为此 Host |
| `upos-sz-mirroraliov.bilivideo.com` | 阿里云 | 海外 | |
| `upos-sz-mirrorcosov.bilivideo.com` | 腾讯云 | 海外 | |
| `upos-sz-mirrorhwov.bilivideo.com` | 华为云 | 海外 | |
| `upos-sz-mirrorcf01ov.bilivideo.com` | Cloudflare | 海外 | |
| `upos-sz-mirroralibstar1.bilivideo.com` | 阿里云 | 东南亚 | 其他类型不能替换为此 Host |
| `upos-sz-mirrorcosbstar1.bilivideo.com` | 腾讯云 | 东南亚 | 其他类型不能替换为此 Host |
| `upos-sz-mirrorhwbstar1.bilivideo.com` | 华为云 | 东南亚 | 其他类型不能替换为此 Host |
| `upos-bstar1-mirrorakam.akamaized.net` | Akamai | 东南亚 | 有参数校验，其他类型不能替换为此 Host |

带 `ov` = 海外节点，带 `bstar` = 东南亚哔哩哔哩服务。

---

## UPOS 型 CDN

域名 Regex: `upos-sz-estg([0-9,a-z]*).bilivideo.com`

判断方式: URL 参数 `os=upos`

特点: 直接从对象存储取资源，多见于冷门视频。需要回源，速度中等。

已知域名:
- `upos-sz-estghw.bilivideo.com`
- `upos-sz-estgcos.bilivideo.com`
- `upos-sz-estgoss.bilivideo.com`

---

## BCache 型 CDN

域名 Regex: `^cn-.*\.bilivideo\.(com|cn)$`

判断方式: URL 参数 `os=bcache`

特点: 自建机房，域名编码了地理位置和运营商。

命名规则: `cn-{省份缩写}{城市拼音}-{运营商}-{序号}.bilivideo.com`

示例:
- `cn-nmghhht-cu-08-01.bilivideo.com` — 内蒙古呼和浩特，联通
- `cn-hncs-cm-03-08.bilivideo.com` — 湖南长沙，移动

运营商缩写:
- `cu` = 联通
- `ct` = 电信
- `cm` = 移动

质量因地区而异，IPv6 支持较差。四川、浙江宁波、河南等 IDC 多、带宽充足的地方质量较好。

---

## MCDN 型 (PCDN 子类)

域名包含 `mcdn`

判断方式: URL 参数 `os=mcdn`

特点: 质量最差，不稳定，速度慢，但带宽成本低。

注意: `mirrorcoso1` 域名 + `os=mcdn` = 实际是 MCDN，而非 Mirror。

**`os` 参数才是判断 CDN 类型的关键**。

---

## IP:Port 型 (PCDN 子类)

格式: `http://IP:Port/v1/resource/*`

特点: APP 端未开启 HTTPS 视频流时出现，首次发现于东南亚区域。

---

## 免流域名

格式: `(upos|proxy).*-tf-.*.bilivideo.com`

示例: `proxy-tf-all-ws.bilivideo.com`

特点: **不认 upsig 鉴权**，用于免流场景。替换不影响免流。

---

## 已废弃的 Mirror CDN

| 域名 | 厂商 | 状态      |
|---|---|---------|
| `upos-sz-mirrorcoso2.bilivideo.com` | 腾讯云 VOD | 废弃，无解析  |
| `upos-sz-mirrorbos.bilivideo.com` | 百度云 | 有解析但不可用 |
| `upos-sz-mirrorwcs*.bilivideo.com` | 网宿 | 废弃      |
| `upos-sz-mirrorks*.bilivideo.com` | 金山云 | 废弃      |
| `upos-sz-mirrorkodo*.bilivideo.com` | 七牛云 | 废弃      |

---

## 可替换性

> 仅供参考，实际情况可能随时变动
> 测试时间 2026-06-18

| 源类型 | 可替换为 Mirror | 备注 |
|---|---|---|
| BCache | ✓ | |
| UPOS | ✓ | |
| 部分 MCDN | ✓ | |
| `IP:Port/v1/resource/*` | ✗ | 缺少 trid 参数，只能用免流域名代理 |
| 海外 Mirror aliov / cosov | ✓ | 实测 upsig 未绑定域名 |
| 海外 Mirror hwov / cf01ov | ✗ | DNS 解析失败，不可达 |
| 东南亚 bstar (`*bstar*`) | ✗ | upsig 绑定域名，返回 404 |
| Akamai (`*akamaized.net`) | ✗ | 强参数校验，返回 403 |

**替换规则**: BCache/UPOS/部分 MCDN 的 playurl 可直接替换 Host 为 Mirror 型。海外 Mirror 中仅 aliov、cosov 可互换，hwov、cf01ov DNS 不可达。

---

## B站调度策略

B站调度策略是"主流给最劣质 PCDN，备份流给稍好的和一个 Mirror"：

1. **主流**: MCDN（质量最差）
2. **备份 1**: 稍好的 MCDN 或 BCache
3. **备份 2**: Mirror（质量最好）

主流不可用时自动 fallback。

在部分区域和劣质运营商网络（如两广地区、移动宽带），非核心 UP 主的视频可能主流和备份都是 MCDN。

---

## 访问控制

### GeoIP

**B站 CDN 无 GeoIP 封禁**。海外 IP 可正常访问国内 Mirror CDN（返回 200）。

此前 403 因测试时使用了 curl 默认 UA，实际触发的是 UA 检查而非 IP 拦截。

### User-Agent 检查

| User-Agent | 结果 |
|---|---|
| Chrome 149 (Win) | 200 ✓ |
| Windows-Media-Player | 200 ✓ |
| NSPlayer (VRChat) | 200 ✓ |
| libmpv | 200 ✓ |
| wget/1.24 | 200 ✓ |
| foobar2000 | 200 ✓ |
| ExoPlayer | 200 ✓ |
| curl/8.7.1 | 403 ✗ |
| VLC/3.0.20 | 403 ✗ |

### upsig 签名绑定

| CDN 类型 | 域名互换 | 说明 |
|---|---|---|
| 国内 Mirror | ✓ 可互换 | upsig 不绑定域名，可自由轮询 |
| 海外 Mirror aliov / cosov | ✓ 可互换 | 实测 upsig 不绑定 (2026-06)，但 B站随时可能收紧 |
| 海外 Mirror hwov / cf01ov | 不可达 | DNS 无法解析 |
| 东南亚 bstar | ✗ 不可互换 | upsig 绑定域名，返回 404 |
| Akamai | ✗ 不可互换 | 强参数校验，返回 403 |
| BCache → Mirror | ✓ 可替换 | |
| UPOS → Mirror | ✓ 可替换 | |

## CDN 升级策略

实现见 `src/mirror_cdn.rs` `upgrade_cdn_hostname()`，按优先级依次判断：

1. **免流 proxy-tf** → 放行（不认 upsig 鉴权）
2. **海外 CDN** → 替换为国内 Mirror 轮询（海外节点 upsig 绑定不可靠）
3. **国内 Mirror + os=mcdn** → 替换为国内 Mirror（域名是 Mirror，实际是 MCDN）
4. **国内 Mirror** → 放行（已是最优）
5. **MCDN IP:Port / MCDN 域名 + /v1/resource 路径** → 替换为 proxy-tf 免流代理
6. **其余 (BCache / UPOS / 其他)** → 替换为国内 Mirror 轮询

国内 Mirror 轮询列表（13 个）：ali, alib, alio1, bd, cos, cosb, coso1, hw, hwb, hwo1, 08c, 08h, 08ct
