---
title: "Converter build do pacote LG (.ipk)"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** webos/build.py
**Destino:** rust/crates/freemytv-core/src/package.rs (ipk, ar, tar_gz) + `freemytv webos-build`

Gerar o `.ipk` (ar com debian-binary, control.tar.gz e data.tar.gz) sem as ferramentas da LG.

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
