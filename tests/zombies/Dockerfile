FROM python:3.7-slim-stretch

COPY tests/zombies/ /etc/supermon/
COPY target/debug/supermon/ /bin/supermon

CMD ["/bin/supermon", "/etc/supermon/spec.yml"]
