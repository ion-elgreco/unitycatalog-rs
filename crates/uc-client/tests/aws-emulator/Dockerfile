# AWS Emulator, a local emulator of the AWS APIs, for `just test-aws-emulator`.
# AWS publishes packages, not an image:
# https://docs.aws.amazon.com/emulator/latest/userguide/getting-started-docker.html
FROM public.ecr.aws/amazonlinux/amazonlinux:2023-minimal
# A version of https://downloads.aws.com/aws-emulator/packages/aws-emulator/LTS/versions.json,
# without its last number.
ARG VERSION=1.0.322
# x86_64 or aarch64.
ARG ARCH=x86_64
ADD https://downloads.aws.com/aws-emulator/packages/aws-emulator/LTS/aws-emulator-${VERSION}-1-${ARCH}.rpm /tmp/aws-emulator.rpm
RUN rpm -i /tmp/aws-emulator.rpm && rm /tmp/aws-emulator.rpm
EXPOSE 9297
ENTRYPOINT ["aws-emulator", "-b", "0.0.0.0"]
