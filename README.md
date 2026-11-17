# Rust Edition

Motor em Rust em desenvolvimento para carregar o conteúdo e os mundos do Terraria Mobile 1.4.5.8.6 a partir dos arquivos da cópia do jogador.

## Recursos originais

O leitor inicial lista os arquivos internos de `assets/bin/Data/data.unity3d`, incluindo `resources.assets` e `resources.assets.resS`.

```sh
cargo run --locked --bin unityfs_index -- /caminho/data.unity3d
```

O comando roda no computador e lê o índice do pacote sem alterar o arquivo original. A decodificação de texturas ainda está pendente.

## Mundos do Terraria

O leitor Rust abre em modo somente leitura o cabeçalho e a seção de tiles de arquivos `.wld` no formato 326. Ele preserva IDs, frames, paredes, líquidos e flags do mapa para uso pelo motor.

```sh
cargo run --locked --bin inspect_world -- /caminho/mundo.wld
```

## Prévia do mapa no Android

Extraia o arquivo `.wld` do ZIP, abra o Rust Edition e toque em **Abrir mundo (.wld)**. Arraste para navegar e use o gesto de pinça para ampliar. O app lê o arquivo selecionado em modo somente leitura.

A prévia usa cores de diagnóstico e reduz o mapa para caber em 2.048 pixels no eixo maior. O pacote original organiza tiles e paredes em atlas próprios, cuja integração com o renderizador está pendente. A build ainda não simula o jogo dentro do mundo importado.

## Compilar

Requer Rust estável, JDK 17, Gradle 8.9 e Android SDK. Defina `ANDROID_HOME` ou aponte `sdk.dir` em `android/local.properties`.

```sh
rustup target add aarch64-linux-android
sdkmanager 'platforms;android-35' 'build-tools;34.0.0' 'ndk;27.2.12479018' 'cmake;3.22.1'
cargo test --locked
gradle -p android :app:assembleDebug
```

Saída: `android/app/build/outputs/apk/debug/app-debug.apk`. No GitHub Actions, a compilação disponibiliza o APK no artefato **Rust-Edition-Android-ARM64**.

Para validar no aparelho, abra um mundo de formato 326 e confirme que o mapa aparece e responde a arrastar e ampliar.nquanto segura uma seta, altere dois blocos, saia pela tela inicial e reabra. Confira a posição e as edições. A frequência de simulação é fixa e o desempenho de renderização deve ser medido no dispositivo.
