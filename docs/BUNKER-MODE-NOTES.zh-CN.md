# 掩体模式：思考与决策记录

2026-10-27 · 记录从"Vela 要不要支持掩体模式"到 Airkeys 落地的讨论与决定。

## 结论

掩体模式不进 Vela，而是做成独立的冷钱包 Airkeys。它只对外交换地址，派生路径另起一级 `65738275'`；资产面板只提示多链暴露，不替用户自动转走。

| 问题 | 决定 | 一句话理由 |
| --- | --- | --- |
| Vela 要不要支持掩体模式 | 不支持 | 1-of-n 的 Safe 里，任何一把公开的 passkey 都能单独签名，加一个 EOA owner 对量子安全的收益为零 |
| 掩体钱包放在哪 | 独立的 Airkeys，不连 Vela | Vela 五个端都不用改，"无助记词"的产品原则也不冲突 |
| 用现成硬件钱包行不行 | 不行 | 主流接入方式都会把公钥或 xpub 交给联网电脑 |
| 助记词复用怎么办 | 改路径，不加警告 | 路径前缀 `65738275'` 不在任何标准钱包导出 xpub 的子树里 |
| 一条 EVM 链发送会暴露全部 EVM 链 | 发送前提示，之后在首页逐链转移 | 一次签所有链的请求又长又怪，一条链签一次更好用 |
| 价格从哪来 | Chainlink 喂价 + Vela 的汇率服务 | 两种请求都不带地址 |

## 背景：掩体模式是什么

量子计算机能从公钥推出私钥，但推不出地址背后的公钥。比特币和以太坊的地址都是公钥的哈希，所以一个从没签过名的地址，公钥还没公开，这就是"掩体"。

- **第一次签名就暴露公钥**：转账、消息签名、连接 dApp 都算。之后这个地址上剩下的资产就不在掩体里了。
- **掩体只是争取时间**：最后撤离的那次签名仍会暴露公钥，交易在 mempool 里等待确认的几秒到几分钟是最后的窗口。
- **时间线**：2026 年 3 月 Google Quantum AI 与以太坊基金会等的论文把破解 secp256k1 的估计降到 50 万物理比特以下、至少约 9 分钟；2026 年 9 月 IonQ 给出约 2 万物理比特、每次尝试约 26 天，并自称 2028 年前后有硬件（厂商说法，未经独立审查）。现在的芯片只有一百个比特左右：估计在快速下降，但还没有能跑这个算法的机器。
- **P-256 和 secp256k1 一样脆弱**：passkey 不会因为用了另一条曲线就更安全。

来源：[IonQ 博客](https://ionq.com/blog/the-first-full-stack-blueprint-for-breaking-256-bit-elliptic-curve-signatures) · [The Quantum Insider](https://thequantuminsider.com/2026/09/08/ionq-estimates-20000-qubit-machine-could-break-bitcoin-secp256k1-26-days/) · [Pebblous（Google/EF 3 月估计）](https://blog.pebblous.ai/project/QuantumCrypto/quantum-bitcoin-threat/en/)

## Vela 为什么做不到掩体，以及为什么不在 Vela 里加

起因是一位朋友的观察：Vela 只用 passkey，而 passkey 的公钥按设计是公开的。这个观察是对的：

- 创建钱包时，每把 key 的公钥和 credential ID 都写进公开的 registry 合约。
- 就算不写 registry，P-256 签名本身也会暴露公钥。Vela 自己就靠这一点：`webauthn.rs` 从两次签名里恢复公钥，单 key 钱包就是这样重建的。

但"给 Vela 加一个 EOA 签名者"这个解法不成立：

1. **阈值是 1。** 任何一把 key 都能单独签名，账户的安全程度取决于最弱的那把。给现有钱包加 EOA owner，攻击者用已公开的 passkey 就够了。
2. **要成立就得另做一种账户。** 一个只由离线 EOA 拥有的 Safe，只收不付，一生只撤离一次。合约一行不用改（Safe 原生支持 EOA owner，SafeOp 哈希就是 EIP-712），但 Vela 这边要铺到五个端：owner 类型、新的地址推导、ECDSA 签名格式、扫码签名流程、相机管线、金库账户的各种禁用。
3. **投入换回的保护太少。** 它保护不了现有的 Vela 账户；需要掩体的人不是 Vela 现在的用户；抗量子的撤离目标还不存在；而 Vela 的 iOS/Android 上架、自有代码审计还排在前面。

所以决定：掩体钱包独立做，不连 Vela。Vela 对它而言只是一个普通的收款方。

## 现有硬件钱包的问题：接入协议暴露公钥

私钥都安全地留在设备里，漏出去的是同步那一步。源码核对过：

| 接入方式 | 交给联网电脑的东西 | 证据 |
| --- | --- | --- |
| Keystone（MetaMask 的 QR 接入） | 账户级 xpub，长期存进 keyring | `@keystonehq/base-eth-keyring`：`HDKey.fromExtendedKey(xpub)`，`serialize()` 保存 xpub |
| Trezor（经 MetaMask） | 公钥 + chain code，即 xpub | `@metamask/eth-trezor-keyring`：`getPublicKey` 后存 `publicKey`、`chainCode` |
| Ledger ETH app | 每个地址都连同公钥一起返回 | `@ledgerhq/hw-app-eth`：`getAddress` 的返回值总带 `publicKey` |
| Trezor 设备协议本身 | 可以只给地址 | `EthereumGetAddress` 只返回地址，xpub 要另调 `EthereumGetPublicKey` |

xpub 比单个公钥更糟：量子计算机只要破解一次 xpub 里的公钥，就得到这个节点的私钥，再配上 chain code，它下面所有非 hardened 子地址就都失守了。

经典安全模型里，xpub 泄露只损失隐私，所以没人认真保护它；到了量子时代，它等同于整个账户的私钥。这也意味着标准的 UR / ERC-4527 同步流程不能用于掩体，这是自己写冷钱包的理由。

## Airkeys 的设计原则

Airkeys 是跑在离线手机上的冷钱包（Android、iOS，共用 Rust 核心），配一个只持有地址的 web 资产面板，两者通过 Lumen Code 动态码通信，消息都是可读的 JSON。

- **只给地址**：手机只导出 `{network, index, address}`，没有公钥、xpub、chain code，也没有路径。
- **每一级都 hardened**：一个账户撤离后暴露了公钥，牵连不到兄弟账户。
- **路径另起一级 `65738275'`**：hardened 隔得开兄弟，隔不开祖先。标准钱包导出 xpub 的位置正好是 `m/44'/60'/0'`（Keystone）和 `m/84'/0'/0'`（比特币观察钱包），旧路径 `m/44'/60'/0'/0'/i'` 就在它们下面。加了前缀后，路径是 `m/65738275'/84'/0'/0'/0'/i'` 和 `m/65738275'/44'/60'/0'/0'/i'`；`65738275'` 是 "AIRK" 的 ASCII 十进制码，沿用 BIP-85 把 "SEED" 写成 `83696968'` 的惯例。这样同一个助记词在别的钱包里用过也没关系，导入页不需要任何警告。唯一的例外是导出了根 xpub（深度 0）的工具。
- **只签转账，交易由手机自己组装**：请求里只有可读字段，不带交易字节；不签消息、不连 dApp、不签合约调用；未知字段一律拒绝。
- **助记词只在内存里**，强制断网（实测过 MIUI 的"半开"蓝牙）。
- **网络只做 BTC 和 ETH/EVM**：Solana 的地址就是 Ed25519 公钥本身，原理上做不了掩体。
- **和 Keyper 分工**：Keyper（离线、什么都不存，用邮箱加一句话推导出标准 BIP-39 助记词）负责生成和保管助记词，Airkeys 负责签名。

## 掩体纪律：怎么花钱才不破坏掩体

- **一次签名，所有 EVM 链一起暴露**：同一个地址在所有 EVM 链上相同。决定是只提示、不自动转走：第一次发送前，web 会说"它在 Base（≈ $X）等另外 N 条链上还有资产"；发送之后，首页的 **To move** 逐条列出，每条链一个 Move 按钮，一次签名。
- **Move 是一个不付给任何人的请求**：先转走所有代币，再转走扣除手续费后的原生币，目标是一个公钥未暴露的新账户（比特币是所有币一个输出）。新账户只按序号传给手机，地址由手机自己推导，联网端没法偷换。
- **"没用过"由联网端判断**：联网端本质上就是资产面板，这个判断理应由它做；手机无法离线验证，文档已写明。
- **gas 不能给低**：交易卡在 mempool 里时公钥已暴露、钱还没走，是最危险的窗口。L2 的 L1 数据费按链上实时数据预留了余量。

## 已落地的改动与验证状态

| 改动 | 位置 | 验证 |
| --- | --- | --- |
| 派生路径加前缀 `65738275'` | `core/wallet`、PROTOCOL §3、RECOVERY、DESIGN | 新测试向量由 Foundry `cast` 和 `@scure/bip32` 独立算出；先用两者重算旧向量，逐字一致 |
| 新首页：总额、Bunker / Exposed、To move、跨账户资产列表 | `app-web/airkeys.app` | 浏览器里用真实链上数据看过桌面和手机宽度 |
| "全部转走"请求（`moveAll`） | `chains/evm.ts`、`chains/bitcoin.ts` | 单元测试；对着 Base 主网组装出正确请求（含 L1 数据费） |
| 发送前的多链提示 | `Send.svelte` | 浏览器中验证 |
| 价格：Chainlink + vela-currency，只给列表里的资产定价 | `value.ts`、`prices.svelte.ts` | 喂价地址逐个上链核对（描述、精度、更新时间） |
| 完整端到端 | `scripts/e2e.mjs` | 通过：真实 Rust 核心派生新地址 → 扫码 → 签名 → 广播 → 在主网分叉链上执行 |

顺带发现 Vela 四个端的主网 Chainlink 表里有一条不存在的 BNB 喂价（`0x14e6…5d25`），已删除，提交在本地分支 `fix/dead-bnb-mainnet-feed`，未推送。BNB 一直由 BNB 链自己的喂价定价，用户看到的价格不变。

Airkeys 的改动都还没有提交。

## 遗留问题与重新评估的条件

- **手机 app 要重新构建**：旧版本仍按旧路径派生地址；旧路径上如有测试资金，要先转出。
- **没跑的测试**：比特币恢复配方需要 `bitcoind`；iOS UI 测试和 Android 没有跑。
- **根 xpub**：任何导出了根 xpub 的工具都会让所有路径失守，只能靠"离线推导根密钥"的纪律。
- **抗量子的撤离目标**：链上还不能验证后量子签名，掩体之后撤到哪里仍是空白。
- **何时重新评估 Vela 侧的支持**：链上出现可用的后量子签名验证（precompile 或审计过的合约）、认证器开始支持后量子算法，或出现可信的硬件里程碑。到那时 Vela 该做的是"后量子签名者作 owner"加一键迁移，而不是 EOA 掩体。
