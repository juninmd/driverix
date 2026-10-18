# Driverix

Detecta o hardware do Windows e atualiza drivers pelo **Windows Update**, com ponto de restauração, backup dos drivers atuais e rollback. Irmão do [Faxina](https://github.com/juninmd/faxina): mesma stack (Tauri 2, Rust, React 19, Tailwind v4).

## Por que existe

Ferramentas como Driver Easy, CCleaner e Driver Booster fazem o mesmo trabalho (inventariar PnP, achar driver mais novo, instalar), mas com problemas recorrentes: instalação atrás de paywall, instaladores embutidos, casamento de driver instável e nenhum caminho de volta. O Driverix faz o mesmo trabalho com as regras opostas:

- **Só fontes oficiais.** O Windows Update (`Type='Driver'`) é a única fonte, com assinatura verificada pela Microsoft. Nada de binário baixado de mirror.
- **Nunca faz downgrade.** Versão é comparada como tupla numérica (não texto); data só desempata quando a versão não é comparável.
- **Reversível.** Ponto de restauração → exporta o pacote atual (`pnputil /export-driver`) → instala → verifica `ConfigManagerErrorCode`. Se qualquer passo anterior à instalação falhar, nada é instalado. A aba Backups restaura o pacote anterior.
- **Sem pegadinhas.** Sem paywall, sem telemetria, sem reinício automático, sem aceitar EULA por você.

A pesquisa por trás dessas escolhas (o que é público sobre como os concorrentes funcionam, o que não é) está em [`docs/RESEARCH.md`](docs/RESEARCH.md). Nenhum binário proprietário foi baixado, executado ou decompilado.

## Uso

```
bun install
bun tauri dev        # instalar/restaurar exigem terminal elevado
```

Buscar não exige admin. Instalar e restaurar exigem; o app oferece “Reabrir como administrador”.

CLI somente leitura, útil para diagnosticar:

```
cargo run --manifest-path src-tauri/Cargo.toml --bin driverix-cli -- devices   # inventário
cargo run --manifest-path src-tauri/Cargo.toml --bin driverix-cli -- scan --json
```

## Limites conhecidos

- **Cobertura = o que a Microsoft publica.** Drivers de GPU lançados ontem pela NVIDIA/AMD/Intel podem não estar no Windows Update. Fontes de fabricante ficaram de fora de propósito: os endpoints são não documentados ou com termos que restringem automação (detalhes em `docs/RESEARCH.md`).
- **Instalação e rollback foram testados com um `Host` falso** (ordem das etapas, abortos, manifesto), não ainda numa máquina real elevada. Faça um teste manual antes de confiar neles.
- Só Windows 10 1607+ / 11 (usa `pnputil /export-driver`). Drivers inbox (não-`oem*.inf`) não são exportáveis; para eles o ponto de restauração é o caminho de volta.

## Roadmap

1. Teste manual elevado do ciclo instalar → verificar → restaurar, e um e2e que o automatize numa VM.
2. Fonte opcional: Microsoft Update Catalog (após confirmar os termos de uso).
3. Atualização automática do app (updater assinado) e pipeline de release como no Faxina.

## Desenvolvimento

Veja [`AGENTS.md`](AGENTS.md) para comandos, layout e invariantes de segurança. Gates:

```
bun run lint && bun run typecheck && bun run test
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

MIT.
