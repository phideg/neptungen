# Testing the examples

The examples are used to test neptungen features. Unfortunately testing the sync features requires an FTP/SFTP server.

## FTP server setup

```bash
docker pull delfer/alpine-ftp-server
```

Start the ftp server as a docker container. User name is `neptun` and password `neptun`, too.

```bash
docker run --rm -d --network=host --name ftp -e USERS="neptun|neptun" delfer/alpine-ftp-server
```

> Remark: With this kind of docker execution no data will remain after the container was stopped.

> Hint: Here the ftp server is only intended to run on the localhost. If you leave the `--network=host` option you will not be able to access the ftp server as expected. More info about the docker container and its options can be found [here](https://github.com/delfer/docker-alpine-ftp-server)

The FTP server can be stopped with the following docker command

```bash
docker container kill ftp
```
