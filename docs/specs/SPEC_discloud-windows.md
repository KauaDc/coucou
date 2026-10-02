# SPEC — Integração Discloud (app Windows)

## Objetivo
Quem hospeda bots e apps na [Discloud](https://discloud.com) quer saber, sem abrir o painel, se os apps estão no ar, e poder religá-los na hora quando caem. A integração Discloud entra ao lado de Vercel, GitHub, Stripe etc. na ilha do Coucou para Windows (`windows/`) e faz três coisas:

1. **Monitora** todos os apps da conta: online/offline, CPU e RAM.
2. **Avisa** quando um app cai (badge vermelho + som de erro) e quando volta (badge verde + som de sucesso).
3. **Age**: botões **Reiniciar**, **Parar** e **Iniciar** por app, e um visor dos logs recentes.

Fora do escopo: app macOS (`NotchBuddy/`), upload/commit de código, variáveis de ambiente, snapshots, domínios, bancos de dados, times. Nada disso aparece na ilha.

## API da Discloud (v2)
- Base: `https://api.discloud.app/v2`. Autenticação: header `api-token: <token>`.
- O token sai do painel da Discloud (Dashboard → API) ou do comando `.api` do bot deles. Ver https://docs.discloud.com/faq/general-questions/how-can-i-get-my-discloud-api-token.md

| Uso no Coucou | Método e caminho | Campos usados |
|---|---|---|
| Lista de apps (nome, online) | `GET /app/all` | `apps[].id`, `name`, `online`, `ramKilled`, `exitCode`, `ram` |
| Métricas | `GET /app/all/status` | `apps[].id`, `container`, `cpu`, `memory`, `startedAt` |
| Logs de um app | `GET /app/{id}/logs` | `apps.terminal.small` (fallback `big`), `apps.terminal.url` |
| Iniciar | `PUT /app/{id}/start` | `status`, `message`, `appStatus.online` |
| Parar | `PUT /app/{id}/stop` | `status`, `message` |
| Reiniciar | `PUT /app/{id}/restart` | `status`, `message` |

- Com `{id} = all`, `apps` é uma **lista**; com um id, é um **objeto**. O parser aceita os dois.
- Erro 401 vem como `{ code, message }`; 404 como `{ status, message }`. O limite de requisições **não está documentado**: o poll fica em 60 s e a 429 é tratada.
- Os formatos acima vêm da documentação pública (consultada em 2026-10-02). A Etapa 1 do TODO confere tudo com um token real antes de escrever a UI.

## Comportamento

### Configuração
- Nos Ajustes, uma linha nova **Discloud** com um campo "Token da API" (secreto). O token vai para o Credential Manager como `discloud-token`, nunca para disco nem para o front.
- A integração começa **desligada** (fora de `DEFAULT_SETTINGS.activeIntegrations`). Sem token ou desligada, **nenhuma** chamada de rede.
- Com a app pausada pela bandeja (`PAUSED`), nada de poll **e nada de ação**.

### Poll
- Começa 10 s depois de abrir e repete a cada **60 s**: `GET /app/all` + `GET /app/all/status` (2 requisições por ciclo). Se a segunda falhar, o card mostra os apps sem CPU/RAM.
- Ordenação: offline primeiro, depois por nome.
- O botão **Atualizar** da ilha faz um poll imediato (`refresh_integration`, já existente).

### Eventos (badge + som)
- O Rust guarda o último `online` de cada app. O **primeiro poll só preenche** o card (sem som), como os outros pollers.
- App que passa de online → offline: evento de **erro**, rótulo = nome do app, detalhe = "Ficou offline" (ou "Sem memória" se `ramKilled`, ou "Saiu com código N").
- App que volta offline → online: evento de **sucesso**, detalhe = "De volta ao ar".
- Vários apps mudando no mesmo ciclo: um evento só, o mais grave (erro vence), com o detalhe "+N outros".
- **Mudança causada por uma ação do usuário não toca som de erro**: depois de um clique em Parar, a queda daquele app é esperada e fica silenciosa por até 3 min. Reiniciar e Iniciar também ficam silenciosos (já há feedback no próprio botão).

### Card (visão geral)
Mesmo layout de lista dos outros cards (`header` + `listRow`):
```
● Discloud   Apps
● meu-bot          online · 12% · 180/512 MB
● api-loja         offline
● site             online · 1% · 64/100 MB
```
- Bolinha verde (`online`) ou vermelha (`offline`). No máximo 3 linhas, como os outros cards; acima disso, "+N apps". O cabeçalho mostra "online/total".
- Clique numa linha → abre o detalhe daquele app.

### Detalhe de um app
O card tem ~135 px de altura, então o detalhe tem duas telas:
```
‹  ● meu-bot                    [Online]
   CPU 12% · RAM 180/512 MB · no ar há 3h      (uma linha só, com reticências)
   [Reiniciar] [Parar] [Logs]     (ou [Iniciar] [Logs] se offline)
   Reiniciando… / mensagem da API

‹  ● meu-bot · Logs             Atualizar
   ┌ últimas linhas, monoespaçado, rolável ┐
   └───────────────────────────────────────┘
```
- Os logs são buscados **só ao abrir a tela de logs** (se ainda não houver) e ao clicar em "Atualizar". Não entram no poll. O ‹ da tela de logs volta para as ações.
- **Confirmação inline** para Parar e Reiniciar: o primeiro clique troca o botão por "Confirmar parada?" por 4 s; só o segundo clique chama a API. Iniciar não pede confirmação.
- Durante a ação: botões desabilitados e "Reiniciando…". Depois do retorno, um poll imediato e outro 8 s depois para pegar o novo estado.
- Resultado da ação: a mensagem da API (`message`) aparece sob os botões por 6 s, em verde (ok) ou âmbar (falha ou aviso, ex.: app já no estado pedido); os botões voltam.
- O painel da Discloud abre pelo botão ↗ da ilha (`https://discloud.com/dashboard`).

## Modelo de dados

### Rust → ilha (evento `integration`, `id = "integration_discloud"`)
```json
{
  "apps": [
    { "id": "123abc", "name": "meu-bot", "online": true, "ramKilled": false,
      "exitCode": 0, "ramLimit": 512, "cpu": "12%", "memory": "180/512MB",
      "startedAt": "2026-10-02T09:00:00Z" }
  ]
}
```

### Novos comandos Tauri
| Comando | Entrada | Saída |
|---|---|---|
| `discloud_action` | `appId: string`, `action: "start" \| "stop" \| "restart"` | `{ ok: boolean, message: string }` |
| `discloud_logs` | `appId: string` | `{ text: string, url: string \| null }` ou erro |

- `action` fora da lista → recusado no Rust. `appId` validado (só `[A-Za-z0-9_-]`, 1–64 caracteres; nunca `all`), para não montar URL arbitrária nem disparar ação em massa.

## Impactos
- `docs/privacy.html` hoje diz que o Coucou **só lê** dos serviços. Com a Discloud isso deixa de valer: o texto tem que dizer que a Discloud recebe comandos de iniciar/parar/reiniciar **apenas quando o usuário clica** (versões en e fr).
- `docs/INTEGRATIONS.md` e `windows/README.md`: seção da Discloud. `CHANGELOG.md`: entrada.
- Nenhuma dependência nova (`reqwest`, `serde_json` e `keyring` já estão no projeto).
- Os logs podem conter segredos do app do usuário: ficam só na memória da ilha, **nunca** vão para o `log.rs`.

## Casos de borda
- Token inválido → 401 → card idle com "Chave de API inválida (401)".
- Conta sem apps → card "Nenhum app na Discloud".
- 429 → mantém os últimos dados, mostra "Limite da API, tentando de novo" e pula o próximo ciclo.
- Sem internet → mantém os dados antigos com o erro, como os outros pollers.
- App apagado no painel → some da lista sem evento; o estado guardado dele é descartado.
- Ação em app que já está no estado pedido (ex.: Iniciar com ele online) → a API responde com `message`; mostrar como aviso, não como erro.
- Clique duplo/rápido → o botão fica desabilitado enquanto há requisição pendente.
- Integração desligada ou app pausada com o detalhe aberto → botões de ação desabilitados.
- Logs vazios → "Sem logs ainda".
