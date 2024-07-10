FROM debian:latest

RUN apt-get update && apt-get install -y openssh-server curl
RUN mkdir /var/run/sshd
RUN echo 'root:root123' | chpasswd
RUN echo 'PermitRootLogin yes\nSubsystem sftp internal-sftp' > /etc/ssh/sshd_config
RUN echo 'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMclBYbzN2ePhWt6N2As3D6NahZrU9tFUqdpa2G5moDp kanpov@localhost.localdomain' > /root/.ssh/authorized_keys

EXPOSE 22

CMD ["/usr/sbin/sshd", "-D"]