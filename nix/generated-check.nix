{
  callPackage,
  runCommand,
  clihatch,
}:

let
  render =
    builtins.replaceStrings
      [ "{{name}}" "{{name_snake}}" "{{description}}" "{{author}}" "{{owner}}" "{{year}}" ]
      [ "nix-smoke" "nix_smoke" "Nix scaffold check" "Nix test" "example" "1970" ];
  license = builtins.toFile "nix-smoke-LICENSE" (
    render (builtins.readFile ../templates/LICENSE.tmpl)
  );
  generated = runCommand "clihatch-generated-source" { nativeBuildInputs = [ clihatch ]; } ''
    clihatch new nix-smoke --no-git --into "$TMPDIR" \
      --description 'Nix scaffold check' --author 'Nix test' --owner example
    cp -R "$TMPDIR/nix-smoke" "$out"
    # The CLI defaults to the current copyright year; normalize this fixture
    # so the generated source derivation stays reproducible across years.
    cp ${license} "$out/LICENSE"
  '';
in
# Build the actual generated sources with the same recipe shipped to users.
# Manifest and lock metadata are available at evaluation time: no import from
# derivation and no network-dependent nested Cargo invocation during tests.
callPackage ../templates/nix/package.nix.tmpl {
  sourceOverrides = {
    manifest = builtins.fromTOML (render (builtins.readFile ../templates/Cargo.toml.tmpl));
    src = generated;
    cargoLock.lockFileContents = render (builtins.readFile ../templates/Cargo.lock.tmpl);
  };
}
