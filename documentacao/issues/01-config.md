---
title: "Converter leitura do config.properties para Rust"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/webapp.py: read_config, config_from_args, GENERIC
**Destino:** rust/crates/freemytv-core/src/config.rs

Ler `config.properties` (comentários, aspas, valores padrão, validação de latitude/longitude) e gerar `window.CONFIG` com as mesmas chaves e a mesma ordem do Python.

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
