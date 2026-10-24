# App Android / Google TV (`app/`)

App Java (nível de linguagem Java 8, Android 8+, targetSdk 33), compilado sem Gradle pelo `build.sh`. Não usa
bibliotecas externas, só o Android SDK. Tem cerca de 12 mil linhas em 36 classes no pacote `io.freemytv`.

## Processos

| Processo | O que roda | Por quê |
|---|---|---|
| principal | `HomeMode`, `ModeJob`, `HomeReceiver`, `Adb`, `ScreensaverAlone` (Android 11) | As trocas de modo continuam funcionando mesmo se a tela travar |
| `:home` | `HomeActivity`, `SettingsActivity`, `Screensaver`, a interface | Se travar, o `Watchdog` devolve a tela original |
| `:keys` | `RemoteKeyService` | O sistema espera por ele a cada tecla, então ele não pode ficar lento |

## Componentes por responsabilidade

### Modo e sistema (controle da TV)
| Classe | O que faz |
|---|---|
| `HomeMode` | Alterna entre Original e Free My TV: troca a home, o protetor de tela e desativa (`pm disable-user`) a lista `DEBLOAT`. Protege apps de sistema (`PROTECTED`, `CORE`, `FACTORY`) e apps que salvaram rede Wi-Fi |
| `Adb` | Cliente do protocolo ADB que se conecta à própria TV (`127.0.0.1:5555`) com chave RSA própria. Só o `HomeMode` usa, com comandos fixos |
| `ModeJob` | Roda as trocas como job em segundo plano |
| `HomeReceiver` | Reage ao boot e à atualização do app para conferir o modo |
| `Watchdog` | Volta para a home original após 3 travamentos em 5 minutos |
| `RemoteKeys`, `RemoteKeyService` | Botões de app do controle (Netflix, YouTube...) por marca, via serviço de acessibilidade que só filtra teclas |

### Dados
| Classe | O que faz |
|---|---|
| `HomeData` | Clima e nascer/pôr do sol (Open-Meteo), cotações (Binance), com cache |
| `Recs` | Sugestões da hora (TMDB), só de serviços instalados |
| `HomeApps` | Lista de apps, prateleira, "Todos os apps", como abrir cada um |
| `Prefs` | Ajustes feitos na TV |
| `Photos`, `PhotoInbox` | Fotos do usuário: importação, rotação EXIF, corte 16:9, remoção de GPS |

### Interface
| Classe | O que faz |
|---|---|
| `HomeActivity`, `FireHomeActivity` | A tela inicial (e a versão para Fire TV) |
| `HomeView` | Desenha toda a tela num Canvas (cerca de 3.100 linhas) |
| `SettingsActivity` | Tela de configurações |
| `Screensaver`, `ScreensaverAlone`, `DreamView` | Protetor de tela com efeito Ken Burns e relógio |
| `Scene`, `Shaders`, `Frost` | Aurora e "vidro líquido" em shaders AGSL (Android 13+) |
| `Backdrop`, `Shadows`, `Recording` | Fundo da sugestão, sombras para GPUs antigas, gravação de desenho |
| `Ui`, `Symbols`, `Glyphs` | Fontes, traduções (`Ui.tr`), ícones, logos das moedas |
| `Spring`, `Click` | Animação de mola, som de clique |
| `Spoken` | Acessibilidade (TalkBack) para a tela desenhada à mão |
| `App`, `AppLog` | Contexto global e log (`adb logcat -s FreeMyTV`) |

## Testes

Os testes do Android precisam do Android SDK (`android.jar`), que não está neste ambiente. A lógica pura que dá
para isolar (formatação, regras de `HomeMode.mayTurnOff`, parsing) entra na fase de conversão da lógica para Rust,
com testes em Rust. Veja [rust.md](rust.md).
