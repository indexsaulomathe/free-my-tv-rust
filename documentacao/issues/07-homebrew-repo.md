---
title: "Converter gerador do repositório do Homebrew Channel"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/homebrew_repo.py
**Destino:** rust/crates/freemytv-core/src/homebrew.rs + `freemytv homebrew-repo`

Gerar `docs/webos/apps.json` com SHA-256 e tamanho do `.ipk`.

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
