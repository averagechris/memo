{
  description = "Keep decisions, environment quirks, and workflow lessons across agent sessions";

  nixConfig = {
    extra-substituters = ["https://averagechris-dotfiles.cachix.org"];
    extra-trusted-public-keys = ["averagechris-dotfiles.cachix.org-1:VwJkl5dG1+xGDY5x884mH/kVwwpgwBAdBKIF3BZiia4="];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = [
      "aarch64-darwin"
      "aarch64-linux"
      "x86_64-darwin"
      "x86_64-linux"
    ];

    forAllSystems = nixpkgs.lib.genAttrs systems;
    pkgsFor = system: import nixpkgs {inherit system;};
    cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
    package = cargoToml.package;
    mkToolApp = system: name: runtimeInputs: text: let
      pkgs = pkgsFor system;
    in
      pkgs.writeShellApplication {
        inherit name runtimeInputs text;
      };
    ciFmt = system:
      mkToolApp system "ci-fmt" [(pkgsFor system).cargo (pkgsFor system).rustfmt] ''
        cargo fmt --all -- --check
      '';
    ciClippy = system:
      mkToolApp system "ci-clippy" [(pkgsFor system).cargo (pkgsFor system).clippy] ''
        cargo clippy --all-targets --all-features -- --deny warnings
      '';
    ciTest = system:
      mkToolApp system "ci-test" [(pkgsFor system).cargo (pkgsFor system).rustc] ''
        cargo test --all-targets --all-features --quiet
      '';
    ciAudit = system:
      mkToolApp system "ci-audit" [(pkgsFor system).cargo (pkgsFor system).cargo-audit] ''
        cargo audit --deny warnings
      '';
    ciDeny = system:
      mkToolApp system "ci-deny" [(pkgsFor system).cargo (pkgsFor system).cargo-deny] ''
        cargo deny check
      '';
    ciMachete = system:
      mkToolApp system "ci-machete" [(pkgsFor system).cargo (pkgsFor system).cargo-machete] ''
        cargo machete
      '';
    ciSort = system:
      mkToolApp system "ci-sort" [(pkgsFor system).cargo (pkgsFor system).cargo-sort] ''
        cargo sort --workspace --check
      '';
    staticChecks = system:
      mkToolApp system "static-checks" [
        (ciFmt system)
        (ciClippy system)
      ] ''
        ci-fmt
        ci-clippy
      '';
    nixFormatter = system: let
      pkgs = pkgsFor system;
    in
      pkgs.writeShellApplication {
        name = "alejandra";
        runtimeInputs = [pkgs.alejandra];
        text = ''
          if [[ $# -eq 0 ]]; then
            exec alejandra -q .
          fi

          exec alejandra -q "$@"
        '';
      };
  in {
    packages = forAllSystems (system: let
      pkgs = pkgsFor system;
      lib = pkgs.lib;
      app = pkgs.rustPlatform.buildRustPackage {
        pname = "memo";
        version = package.version;
        src = lib.cleanSource ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [pkgs.installShellFiles];

        postInstall = lib.optionalString (pkgs.stdenv.buildPlatform.canExecute pkgs.stdenv.hostPlatform) ''
          $out/bin/memo completions bash > memo.bash
          $out/bin/memo completions zsh > _memo
          $out/bin/memo completions fish > memo.fish
          installShellCompletion --bash --name memo memo.bash
          installShellCompletion --zsh _memo
          installShellCompletion --fish memo.fish
        '';

        meta = {
          description = package.description;
          license = lib.licenses.mitOrApache2;
          mainProgram = "memo";
        };
      };
    in {
      default = app;
      memo = app;
      ci-fmt = ciFmt system;
      ci-clippy = ciClippy system;
      ci-test = ciTest system;
      static-checks = staticChecks system;
      ci-audit = ciAudit system;
      ci-deny = ciDeny system;
      ci-machete = ciMachete system;
      ci-sort = ciSort system;
    });

    apps = forAllSystems (system: {
      default = self.apps.${system}.memo;
      memo = {
        type = "app";
        program = "${self.packages.${system}.memo}/bin/memo";
      };
      ci-audit = {
        type = "app";
        program = "${self.packages.${system}.ci-audit}/bin/ci-audit";
      };
      ci-deny = {
        type = "app";
        program = "${self.packages.${system}.ci-deny}/bin/ci-deny";
      };
      ci-machete = {
        type = "app";
        program = "${self.packages.${system}.ci-machete}/bin/ci-machete";
      };
      ci-sort = {
        type = "app";
        program = "${self.packages.${system}.ci-sort}/bin/ci-sort";
      };
      ci-fmt = {
        type = "app";
        program = "${self.packages.${system}.ci-fmt}/bin/ci-fmt";
      };
      ci-clippy = {
        type = "app";
        program = "${self.packages.${system}.ci-clippy}/bin/ci-clippy";
      };
      ci-test = {
        type = "app";
        program = "${self.packages.${system}.ci-test}/bin/ci-test";
      };
      static-checks = {
        type = "app";
        program = "${self.packages.${system}.static-checks}/bin/static-checks";
      };
    });

    checks = forAllSystems (system: {
      inherit (self.packages.${system}) memo;
    });

    devShells = forAllSystems (system: let
      pkgs = pkgsFor system;
    in {
      default = pkgs.mkShell {
        packages = with pkgs; [
          alejandra
          cargo
          cargo-audit
          cargo-deny
          cargo-machete
          cargo-outdated
          cargo-sort
          clippy
          direnv
          jujutsu
          nixd
          rust-analyzer
          rustc
          rustfmt
          sccache
        ];
      };
    });

    formatter = forAllSystems nixFormatter;
  };
}
