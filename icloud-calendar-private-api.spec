Name:           icloud-calendar-private-api
Version:        1.2.0
Release:        1%{?dist}
Summary:        Expose iCloud Calendar data via a private API

License:        MIT
URL:            https://github.com/antedebaas/%{name}
Source0:        https://github.com/antedebaas/%{name}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.70
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  openssl-devel
BuildRequires:  systemd-rpm-macros
BuildRequires:  pkgconfig(openssl)
BuildRequires:  make

# Only build on supported architectures for Rust
ExcludeArch:    i686 s390 %{power64}

# For COPR compatibility
%if 0%{?fedora} >= 36 || 0%{?rhel} >= 9
%bcond_without check
%else
%bcond_with check
%endif

%global debug_package %{nil}

Requires:       glibc
Requires(pre):  shadow-utils
Requires(post): systemd
Requires(preun): systemd
Requires(postun): systemd
BuildRequires:  cargo
BuildRequires:  cpp
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  make

%description
Expose iCloud Calendar data via a private API

%pre
# Create the icloudcalendarapi system user
getent group icloudcalendarapi >/dev/null || groupadd -r icloudcalendarapi
getent passwd icloudcalendarapi >/dev/null || \
    useradd -r -g icloudcalendarapi -d /var/lib/icloudcalendarapi -s /sbin/nologin \
    -c "iCloud Calendar API Service" icloudcalendarapi
exit 0

%prep
%autosetup -n %{name}-%{version}

%build
# Set build environment for optimal compilation
export CARGO_TARGET_DIR=%{_builddir}/%{name}-%{version}/target
export RUSTFLAGS="-Ccodegen-units=1 -Clink-dead-code=off"

# Ensure we have a proper Cargo.lock
[ -f Cargo.lock ] || cargo generate-lockfile

# Build with release optimizations
cargo build --release --verbose --locked --bin icloud-calendar-private-api --bin icloud-calendar-private-cli

%install
install -d %{buildroot}%{_bindir}
install -d %{buildroot}%{_unitdir}
install -d %{buildroot}%{_sysconfdir}/icloudcalendarapi
install -d %{buildroot}%{_sharedstatedir}/icloudcalendarapi

# Install binaries
install -D -m 755 %{_builddir}/%{name}-%{version}/target/release/icloud-calendar-private-api %{buildroot}%{_bindir}/icloud-calendar-private-api
install -D -m 755 %{_builddir}/%{name}-%{version}/target/release/icloud-calendar-private-cli %{buildroot}%{_bindir}/icloud-calendar-private-cli

# Install systemd service file
install -D -m 644 icloud-calendar-private-api.service %{buildroot}%{_unitdir}/icloud-calendar-private-api.service

# Install example configuration
install -D -m 644 config.example.toml %{buildroot}%{_sysconfdir}/icloudcalendarapi/config.example.toml

%post
%systemd_post icloud-calendar-private-api.service

%preun
%systemd_preun icloud-calendar-private-api.service

%postun
%systemd_postun_with_restart icloud-calendar-private-api.service
# Clean up user and group on full uninstall
if [ $1 -eq 0 ]; then
    userdel icloudcalendarapi 2>/dev/null || :
    groupdel icloudcalendarapi 2>/dev/null || :
fi

%files
%{_bindir}/icloud-calendar-private-api
%{_bindir}/icloud-calendar-private-cli
%{_unitdir}/icloud-calendar-private-api.service
%dir %attr(0755, icloudcalendarapi, icloudcalendarapi) %{_sysconfdir}/icloudcalendarapi
%config(noreplace) %attr(0644, root, root) %{_sysconfdir}/icloudcalendarapi/config.example.toml
%dir %attr(0755, icloudcalendarapi, icloudcalendarapi) %{_sharedstatedir}/icloudcalendarapi

%changelog
* Sat Jan 11 2025 Ante de Baas <antedebaas@users.github.com> - 1.2.0-1
- Added Stalwart authentication support for API endpoints
- Optional HTTP Basic Authentication for /list and /calendar/:name endpoints
- Credentials validated against Stalwart mail server (JMAP/IMAP)
- Public endpoints (/ and /health) remain accessible without authentication
- /list endpoint now returns full URLs in api_url field
- Added public_url and public_path configuration for reverse proxy support
- Reminders/tasks are now filtered out from calendar listings
- Updated dependencies for improved security

* Thu Jan 09 2025 Ante de Baas <antedebaas@users.github.com> - 1.1.0-1
- Calendar endpoint now serves iCal data inline instead of as attachment
- Added support for URL-encoded calendar names (handles spaces and special characters)
- List endpoint now includes API URLs alongside iCloud URLs
- CLI tool now supports --list flag to show calendars with API URLs
- Improved compatibility with calendar applications and browsers

* Tue May 12 2026 Ante de Baas <antedebaas@users.github.com> - 1.0.0-1
- Initial package
