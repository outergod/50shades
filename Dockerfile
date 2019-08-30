FROM communicatio/rust:1.37.0

USER root
RUN dnf install -y openssl-devel dbus-devel glibc-devel \
 && dnf clean all
USER rust
