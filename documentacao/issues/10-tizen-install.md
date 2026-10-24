---
title: "Converter instalador Samsung (protocolo SDB)"
labels: [fase-1-ferramentas, conversao-rust]
status: a-fazer
---

**Origem:** tizen/install.py
**Destino:** `freemytv tizen-install IP [--uninstall]`

Falar o protocolo SDB (porta 26101) diretamente: enviar o `.wgt`, `0 vd_appinstall`, `0 was_execute`.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
