# Rust Edition

Protótipo independente para reimplementar o Terraria Mobile em Rust. A referência de comportamento é o Terraria Mobile 1.4.5.8.6. Este primeiro marco usa um terreno e gráficos de teste próprios.

## Primeiro marco

- Mundo 2D de 160 × 72 blocos com câmera e desenho da área visível.
- Simulação em Rust a 60 passos por segundo, com movimento, gravidade e colisão.
- Controles multitouch. Use as setas para andar, **Pular** e o botão superior para alternar **Quebrar** e **Colocar**. Toque nos blocos próximos para editar.
- Salvamento ao sair da tela do jogo. O arquivo interno RE01 preserva os blocos e a posição do jogador.
- APK de desenvolvimento para Android 8.0+ ARM64.

O próximo marco de compatibilidade é ler uma cópia real de um mundo da versão alvo e comparar os tiles, tipos e frames com os vistos no Terraria original. Este protótipo ainda usa o save RE01 e não implementa `.wld`, assets originais, multiplayer ou mods.

## Compilar

Requer Rust estável, JDK 17, Gradle 8.9 e Android SDK. Defina `ANDROID_HOME` ou aponte `sdk.dir` em `android/local.properties`.

```sh
rustup target add aarch64-linux-android
sdkmanager 'platforms;android-35' 'build-tools;34.0.0' 'ndk;27.2.12479018' 'cmake;3.22.1'
cargo test --locked
gradle -p android :app:assembleDebug
```

Saída: `android/app/build/outputs/apk/debug/app-debug.apk`. No GitHub Actions, a compilação disponibiliza o APK no artefato **Rust-Edition-Android-ARM64**.

Para validar no aparelho, abra o app, mova e pule enquanto segura uma seta, altere dois blocos, saia pela tela inicial e reabra. Confira a posição e as edições. A frequência de simulação é fixa e o desempenho de renderização deve ser medido no dispositivo.
