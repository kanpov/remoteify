FROM debian:latest

RUN apt-get update && apt-get install -y openssh-server curl
RUN mkdir /var/run/sshd
RUN echo 'root:root123' | chpasswd
RUN echo 'PermitRootLogin yes\nSubsystem sftp internal-sftp' > /etc/ssh/sshd_config
RUN echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJxRFH1EoY5rbW/7fS6o3fR30D35A76AxZJZYKcKrC/H kanpov@localhost.localdomain' > /root/.ssh/authorized_keys

EXPOSE 22

CMD ["/usr/sbin/sshd", "-D"]