# Ferramentas (build, instalação e utilitários)

Scripts que rodam no **computador**, nunca na TV. Todos em Python 3 com biblioteca padrão, exceto onde indicado.

## Componentes

### Configuração: `tools/webapp.py`
Código compartilhado pelos builds de LG e Samsung.

| Função | O que faz |
|---|---|
| `config_from_args(args)` | Escolhe a configuração: `--generic` (sem cidade e sem chave, usada nas releases), `--config CAMINHO` ou `config.properties` |
| `read_config(path)` | Lê o `config.properties` (`chave=valor`, `#` é comentário). Cria o arquivo a partir de `config.example.properties` se ele não existir. Valida latitude e longitude como números |
| `coins()` | Extrai os caminhos SVG dos logos das moedas dos drawables Android (`coin_*.xml`) |
| `catalogues()` | Monta as traduções com só as chaves que o app web usa |
| `files(config, made_by)` | Junta tudo o que vai no pacote: `web/*`, `config.js`, `coins.js`, `i18n.js`, símbolos e fonte |
| `preview(app, dir)` | Grava os arquivos numa pasta para abrir `index.html` no navegador |

### Pacote LG: `webos/build.py`
Gera `FreeMyTV-webos.ipk` sem as ferramentas da LG. O `.ipk` é um arquivo **ar** (o formato do `.deb`) com três
membros: `debian-binary`, `control.tar.gz` (metadados) e `data.tar.gz` (o app em `usr/palm/applications/io.freemytv/`).
- `tar_gz(entries)`: cria um `.tar.gz` com dono root, no formato do `ares-package`.
- `ar(members)`: escreve o arquivo ar (cabeçalho de 60 bytes por membro, alinhado em 2 bytes).
- `build(preview, config)`: monta o pacote completo.

### Instalação LG: `webos/install.py`
1. Baixa a chave SSH privada da TV pelo Key Server (`http://IP:9991/webos_rsa`).
2. Destrava a chave com a passphrase digitada (`ssh-keygen -p`) e a guarda em `~/.ssh/freemytv_webos_<IP>`.
3. Copia o `.ipk` por SSH (usuário `prisoner`, porta 9922) para `/media/developer/temp/`.
4. Instala chamando o Luna `com.webos.appInstallService/dev/install` e abre o app.

`--uninstall` chama `.../dev/remove`. Veja os riscos em [seguranca.md](seguranca.md).

### Pacote e instalação Samsung: `tizen/build.py`, `tizen/install.py`
- `build.py`: gera `FreeMyTV-tizen.wgt`, um zip com `config.xml` e o app web. `--sign PERFIL` assina com o CLI `tizen`.
- `install.py`: fala o protocolo **SDB** (o "ADB" da Samsung, porta 26101) diretamente. Envia o `.wgt`, instala com
  `0 vd_appinstall` e abre com `0 was_execute`.

### Repositório do Homebrew Channel: `tools/homebrew_repo.py`
Gera `docs/webos/apps.json` com o SHA-256 e o tamanho do `.ipk` da release. É o que o webOS Homebrew Channel lê
para instalar o app sem computador.

### Fusos horários: `tools/make_zones.py`
Lê o banco de fusos (`/usr/share/zoneinfo/zone.tab` e `tzdata.zi`) e gera `web/zones.js`: para cada fuso, o país, a
latitude, a longitude e a cidade. O app usa isso para descobrir a cidade quando foi montado com `--generic`.
- `degrees("+4230", 2)` converte o formato ±GGMM[SS] em graus decimais.

### Traduções: `tools/translate.py`
- Extrai os textos do código (`Ui.tr(...)` no Java, `tr(...)` no JS) e gera `en.json`.
- `--check` valida todos os catálogos: chaves faltando, placeholders `{nome}` diferentes, formas de plural CLDR.
- `--fill` traduz o que falta pela API da Anthropic (precisa de `ANTHROPIC_API_KEY`), com cache.

### Fotos: `tools/photos.py`
Envia fotos para o protetor de tela do Google TV por `adb push`. Quando o Android não deixa, usa
`content write` no `PhotoInbox` do app.

### Arte e ícones: `tools/make_art.py`, `tools/make_symbols.py` (precisam do Pillow)
Geram o ícone, o banner e os PNGs de símbolos (Material Symbols). Só são usados quando a arte muda.

### Build Android: `build.sh`
`aapt2 → javac → d8 → zipalign → apksigner`, sem Gradle. Gera `Config.java` a partir do `config.properties`.

### CI: `tools/check.sh`
Faz o build Android, roda `translate.py --check`, monta os pacotes LG e Samsung e verifica se `web/` é ES5.

## Equivalentes em Rust

Veja [rust.md](rust.md). A CLI `freemytv` (pasta `rust/`) substitui estes scripts um por um.
