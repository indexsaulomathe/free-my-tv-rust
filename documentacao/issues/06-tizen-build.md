---
title: "Converter build do pacote Samsung (.wgt)"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tizen/build.py (sem --sign)
**Destino:** rust/crates/freemytv-core/src/package.rs (wgt) + `freemytv tizen-build`

Gerar o `.wgt` sem assinatura. A assinatura continua com o CLI `tizen` da Samsung (issue futura, se fizer sentido).

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
