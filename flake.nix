{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    naersk.url = "github:nix-community/naersk";
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
  };
  outputs = { flake-parts, naersk, nixpkgs, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      perSystem = { system, ... }: let
        pkgs = import nixpkgs { inherit system; };
        naersk' = pkgs.callPackage naersk {};
        rip2 = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = [ pkgs.installShellFiles ];
          postInstall = ''
           installShellCompletion --cmd rip \
             --bash <($out/bin/rip completions bash) \
             --fish <($out/bin/rip completions fish) \
             --zsh <($out/bin/rip completions zsh)
          '';
        };
      in
      with pkgs;
      {
        packages.default = rip2;
        devShells.default = mkShell {
          buildInputs = [ rip2 ];
        };
        apps.default = {
          type = "app";
          program = "${rip2}/bin/rip";
        };
      };
    };
}
