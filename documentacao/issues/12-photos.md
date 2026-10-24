---
title: "Converter envio de fotos (photos.py)"
labels: [fase-1-ferramentas, conversao-rust]
status: a-fazer
---

**Origem:** tools/photos.py
**Destino:** `freemytv photos [IP] ARQUIVOS...`

Enviar fotos por `adb push` ou `content write` no PhotoInbox. Depende do `adb` instalado.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
