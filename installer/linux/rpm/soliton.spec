# Packages the release archive built by scripts/build-linux.sh for Fedora.
# Built by scripts/build-rpm.sh, which defines the version, the license
# expression and the date of the last commit; release notes live in the AppStream metadata.
%global app_id io.github.Carlton2001.Soliton
# The binary is built by cargo outside rpmbuild, without separate debug sources.
%global debug_package %{nil}

Name:           soliton
Version:        %{soliton_version}
Release:        1%{?dist}
Summary:        Fast music player for large local collections
# Soliton, the Rust crates linked on Linux and the embedded assets.
License:        %{soliton_license}
URL:            https://github.com/Carlton2001/Soliton
Source0:        Soliton-%{version}-x86_64-linux.tar.gz
ExclusiveArch:  x86_64

BuildRequires:  desktop-file-utils
BuildRequires:  appstream
# GPUI renders through Vulkan; Mesa provides lavapipe as a software fallback.
Requires:       vulkan-loader
Requires:       mesa-vulkan-drivers
# Loaded at run time, so not detected from the binary.
Requires:       libwayland-client.so.0()(64bit)
Requires:       hicolor-icon-theme
# Fallback for characters missing from the embedded Inter and JetBrains Mono.
Requires:       default-fonts-core-sans

%description
Soliton is a fast music player for large local collections. It indexes your
folders in the background, groups albums and their editions, and shows
embedded cover art. Your music files are only ever read.

%prep
%autosetup -n Soliton-%{version}-x86_64-linux

%build
# Prebuilt by cargo in scripts/build-linux.sh.

%install
install -Dm755 bin/soliton %{buildroot}%{_bindir}/soliton
# The per-user marker would let Soliton rewrite the icon; system entries stay fixed.
sed -e 's/^Exec=@EXEC@$/Exec=soliton/' -e '/^X-Soliton-Install=user$/d' \
    share/applications/%{app_id}.desktop > %{app_id}.desktop
install -Dm644 %{app_id}.desktop %{buildroot}%{_datadir}/applications/%{app_id}.desktop
install -Dm644 share/metainfo/%{app_id}.metainfo.xml \
    %{buildroot}%{_metainfodir}/%{app_id}.metainfo.xml
install -Dm644 share/icons/hicolor/scalable/apps/%{app_id}.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{app_id}.desktop
appstreamcli validate --no-net %{buildroot}%{_metainfodir}/%{app_id}.metainfo.xml

%files
%license LICENSE licenses/dependencies
%{_bindir}/soliton
%{_datadir}/applications/%{app_id}.desktop
%{_metainfodir}/%{app_id}.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/%{app_id}.svg

%changelog
* %{soliton_date} Carlton2001 - %{version}-1
- Soliton %{version}; release notes in the AppStream metadata.
