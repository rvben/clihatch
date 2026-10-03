{
  lib,
  stdenv,
  rustPlatform,
  installShellFiles,
  makeWrapper,
  git,
  gh,
  openssh,
}:

let
  manifest = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = manifest.package.name;
  version = manifest.package.version;
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../tests
      ../templates
      ../schemas
      ../README.md
      ../LICENSE
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    installShellFiles
    makeWrapper
  ];
  nativeCheckInputs = [
    git
    openssh
  ];
  doCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform;

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    for shell in bash fish zsh; do
      "$out/bin/clihatch" completions "$shell" > "clihatch.$shell"
    done
    installShellCompletion clihatch.{bash,fish,zsh}
  '';

  # Git initialization and release-secret management call these tools at runtime.
  postFixup = ''
    wrapProgram "$out/bin/clihatch" \
      --prefix PATH : "${
        lib.makeBinPath [
          git
          gh
          openssh
        ]
      }"
  '';

  doInstallCheck = stdenv.buildPlatform.canExecute stdenv.hostPlatform;
  installCheckPhase = ''
    runHook preInstallCheck
    test "$("$out/bin/clihatch" --version)" = "clihatch ${finalAttrs.version}"
    "$out/bin/clihatch" --help > /dev/null
    for completion in \
      "$out/share/bash-completion/completions/clihatch.bash" \
      "$out/share/fish/vendor_completions.d/clihatch.fish" \
      "$out/share/zsh/site-functions/_clihatch"; do
      if ! test -s "$completion"; then
        echo "Missing or empty completion file: $completion" >&2
        exit 1
      fi
    done
    # Exercise the installed wrapper without relying on the build environment's Git.
    PATH=/nonexistent "$out/bin/clihatch" new nix-install-smoke \
      --into "$TMPDIR" --author 'Nix test' --owner example > /dev/null
    test -d "$TMPDIR/nix-install-smoke/.git"
    test -s "$TMPDIR/nix-install-smoke/Cargo.lock"
    test -s "$TMPDIR/nix-install-smoke/flake.lock"
    runHook postInstallCheck
  '';

  meta = {
    inherit (manifest.package) description homepage;
    license = lib.licenses.mit;
    mainProgram = "clihatch";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
  };
})
