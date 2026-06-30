# SSH / Agent Forwarding

SSH 鍵認証は、どの agent を使うかを `$SSH_AUTH_SOCK` に一本化する。
`IdentityAgent` をホストごとに固定すると、多段 SSH で「接続元の agent を使う」
動きが壊れやすいので、原則として共有設定には書かない。

## 基本方針

- ローカル端末では、そのマシンの 1Password SSH agent を `$SSH_AUTH_SOCK` に入れる。
- SSH セッション内では、接続元から転送された `$SSH_AUTH_SOCK` だけを使う。
- SSH セッション内で `$SSH_AUTH_SOCK` が空なら、agent forwarding なしで入っている。
- jump host 側のローカル agent へ自動フォールバックしない。承認 UI が見えない場所で待つため。
- `ForwardAgent yes` は信頼できる jump host だけに限定する。

典型的な経路:

```text
client-machine --ssh -A--> jump-host --ssh--> target-host
```

この場合、`target-host` への署名承認は `client-machine` の 1Password に出る。
`jump-host` の 1Password は使わない。

## Client 設定

client 側で 1Password SSH agent が鍵を返すことを確認する。

```sh
ssh-add -l
```

macOS では 1Password の socket を `$SSH_AUTH_SOCK` に設定する。

```zsh
export SSH_AUTH_SOCK="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"
```

Windows では 1Password for Windows の SSH agent を有効化する。Windows 標準の
`OpenSSH Authentication Agent` service が `\\.\pipe\openssh-ssh-agent` を先に
掴むと 1Password が使われないので、標準 service は停止または無効化する。

client 側の `~/.ssh/config`:

```sshconfig
Host jump-host
  HostName jump-host.example.internal
  User jump-user
  ForwardAgent yes

Host target-host target-host.example.internal
  HostName target-host.example.internal
  User target-user
  ForwardAgent yes
```

`IdentityAgent` はここでは固定しない。OpenSSH が現在の agent を使い、`-A` /
`ForwardAgent yes` で jump host に転送する。

## Jump Host 設定

jump host 側の `~/.ssh/config` は target host の接続先と agent forwarding だけを
持つ。

```sshconfig
Host target-host target-host.example.internal
  HostName target-host.example.internal
  User target-user
  ForwardAgent yes
```

ここにも `IdentityAgent` は書かない。jump host に入ってきた `$SSH_AUTH_SOCK` を
そのまま target host へ渡す。

シェル初期化では、ローカル端末だけ local agent を補完し、SSH セッション内では
補完しない。

```zsh
LOCAL_1PASSWORD_SSH_AGENT_SOCK="$HOME/Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock"

if [[ -z "${SSH_CONNECTION:-}" && -z "${SSH_AUTH_SOCK:-}" ]]; then
  export SSH_AUTH_SOCK="$LOCAL_1PASSWORD_SSH_AGENT_SOCK"
fi
```

この repo の macOS 設定も同じ方針にしている。`home/mac/default.nix` はローカル
端末だけ 1Password socket を補完し、`home/mac/coder.nix` の `dev` helper も SSH
セッション内では補完しない。

## Verification

client で:

```sh
ssh-add -l
ssh -A jump-host
```

jump host に入った後:

```sh
echo "$SSH_AUTH_SOCK"
ssh-add -l
ssh target-host
```

期待値:

- `$SSH_AUTH_SOCK` が `/tmp/ssh-.../agent...` のような forwarded socket になる。
- `ssh-add -l` で client 側の 1Password 鍵が見える。
- target host への接続時、承認 UI は client 側に出る。

設定解決は `ssh -G` で見る。

```sh
ssh -G target-host | grep -E '^(hostname|user|forwardagent|identityagent) '
```

期待値:

- `forwardagent yes`
- `identityagent` が出ない

OpenSSH の詳細ログでは、成功経路は以下のように進む。

```text
get_agent_identities: agent returned ... keys
Offering public key: ... agent
Server accepts key: ... agent
sign_and_send_pubkey: signing using ...
```

`signing using` の後で止まる場合は、client 側の 1Password 承認待ち。

## Troubleshooting

`target-host` でパスワードを聞かれる:

- client から `jump-host` へ `ssh -A jump-host` で入っていない。
- client 側 `~/.ssh/config` の `ForwardAgent yes` がない。
- jump host 側の `$SSH_AUTH_SOCK` が空。
- target host の `authorized_keys` に client 側 1Password 鍵の公開鍵がない。
- target host の sshd が agent forwarding を拒否している。

jump host 上の `$SSH_AUTH_SOCK` が local 1Password socket になっている:

- SSH セッション内で shell init が local agent にフォールバックしている。
- `SSH_CONNECTION` がある場合は local agent を補完しないように直す。

`ssh target-host` が無言で止まる:

- `ssh -vvv target-host true` で段階を見る。
- `sign_and_send_pubkey` の後なら client 側 1Password の承認待ち。
- `Connecting to ... port 22` の前後なら DNS / Tailscale / TCP 到達性の問題。

Windows client で `ssh-add -l` が鍵を出さない:

- 1Password for Windows の SSH agent が無効。
- Windows 標準 `OpenSSH Authentication Agent` service が pipe を掴んでいる。
- 1Password に SSH 鍵が登録されていない、または agent config の対象外。

Windows client で `ssh-add -l` は鍵を出すが、jump host に `$SSH_AUTH_SOCK` が
作られない:

- client 側で実行されている `ssh` と `ssh-add` が同じ OpenSSH 実装か確認する。

  ```powershell
  where.exe ssh
  where.exe ssh-add
  ssh -V
  ```

  1Password SSH agent を使う経路では、Windows OpenSSH
  (`C:\Windows\System32\OpenSSH\ssh.exe`) を優先する。

- client 側の有効設定を確認する。

  ```powershell
  ssh -G jump-host | findstr /i "forwardagent identityagent controlmaster controlpath"
  ```

  `forwardagent yes` が必要。通常は `identityagent` は不要。

- 既存の multiplex / ControlMaster 接続を再利用していないか確認する。
  forwarding なしで作った古い master connection を再利用すると、後から
  `ForwardAgent yes` を足しても agent socket は作られない。一回だけ切り分ける:

  ```powershell
  ssh -S none -A jump-host
  ```

  これで直る場合は、次に `ssh -A jump-host` も試す。`ssh -A` は直るが
  `ssh jump-host` は直らないなら、client 側の `ForwardAgent yes` がその
  Host に効いていない。`ssh -O exit jump-host` が `No ControlPath specified`
  を返す場合も multiplex ではなく、`-A` 明示の有無が差分。

  古い master connection が原因の場合は、古い master connection を閉じるか、
  client 側の `ControlMaster` / `ControlPath` 設定を見直す。

- verbose log で forwarding request を確認する。

  ```powershell
  ssh -vvv -S none -A jump-host
  ```

  認証後に `Requesting authentication agent forwarding` が出るのが期待値。

## Security Notes

Agent forwarding は秘密鍵そのものを jump host にコピーしない。ただし、接続中の
jump host は forwarded agent に署名要求を送れる。信頼できる jump host にだけ
`ForwardAgent yes` を設定し、不要な `Host * ForwardAgent yes` は避ける。
