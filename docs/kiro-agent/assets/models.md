# models（本地向量模型）

> 分析路径：`C:\Users\12925\AppData\Local\Programs\Kiro\resources\app\extensions\kiro.kiro-agent\models`
> 分析日期：2026-03-18

该目录包含本地嵌入模型 `all-MiniLM-L6-v2`，用于 transformers.js 在本地生成代码库 embeddings。

---

## 目录结构

- `README.md`：说明使用 transformers.js 进行本地 embedding。
- `all-MiniLM-L6-v2/`：模型文件目录。

`all-MiniLM-L6-v2/` 包含：

- `config.json`
- `tokenizer.json`
- `tokenizer_config.json`
- `special_tokens_map.json`
- `vocab.txt`
- `onnx/`（ONNX 权重）

---

## 用途

- 本地生成代码库向量。
- 支持离线语义检索，不依赖外部 API。

---

## 架构关系图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                                models                                       │
│                                                                              │
│  all-MiniLM-L6-v2                                                            │
│   config/tokenizer/vocab/onnx                                                │
│                                                                              │
│             ├──────────────────────────────────────────────────────────────► │
│             │                                                                │
│             │          ┌──────────────────────────────────────────┐          │
│             │          │ transformers.js                           │          │
│             │          │ feature-extraction → embeddings           │          │
│             │          └──────────────────────────────────────────┘          │
│             │                                                                │
│             │  ◄────────────────────────────────────────────────────────────┤
│             │  向量输出 → codebase 检索 / context retrieval                    │
└──────────────────────────────────────────────────────────────────────────────┘
```
