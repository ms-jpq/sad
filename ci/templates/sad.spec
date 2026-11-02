Name:           sad
Version:        {{ version }}
Release:        1%{?dist}
Summary:        Space Age seD
URL:            https://github.com/ms-jpq/sad
License:        MIT
Source0:        https://github.com/ms-jpq/sad/archive/refs/tags/v%{version}.tar.gz

# BuildRequires: List all packages required to build the software
BuildRequires:  git
BuildRequires:  python3
BuildRequires:  curl
BuildRequires:  gcc

%define debug_package %{nil}

# Optional dependencies
# For TUI usage
#Requires:      fzf
# For diff colourizer
#Requires:      diff-so-fancy
#Requires:      git-delta

%description
Space Age seD

%prep
%setup -q
%ifarch aarch64
sed -i '/target.aarch64-unknown-linux-gnu/,+1d' .cargo/config.toml
%endif

%build
# Install Rust using curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
export PATH="$PATH:$HOME/.cargo/bin"
$HOME/.cargo/bin/cargo build --release --all-features

%install
# You may need to adjust paths and permissions as necessary
install -D -m 755 target/release/%{name} %{buildroot}/usr/bin/%{name}
install -D -m 644 LICENSE %{buildroot}/usr/share/licenses/%{name}/LICENSE
install -D -m 644 README.md %{buildroot}/usr/share/doc/%{name}/README.md

%check
$HOME/.cargo/bin/cargo test --release --locked --all-features

%files
# List all installed files and directories
%license LICENSE
%doc README.md
/usr/bin/%{name}

%changelog
* Sun December 8 2024 - Danie de Jager - 0.4.31-3
* Wed October 2 2024 - Danie de Jager - 0.4.31-2
* Sat June 29 2024 - Danie de Jager - 0.4.31-1
* Sun May 26 2024 - Danie de Jager - 0.4.29-1
* Mon May 13 2024 - Danie de Jager - 0.4.28-1
- Initial version
