#!/usr/bin/env bash
# Issue #15 prototype: Recs.franchise and HomeData.parseWeather in Java vs Rust over JNI, on a desktop JVM.
# Needs a JDK 17+ and cargo. Downloads org.json (Android's JSON API) from Maven Central and checks its SHA-1.
set -euo pipefail
cd "$(dirname "$0")"
if [ ! -f json.jar ]; then
  curl -sS -o json.jar https://repo1.maven.org/maven2/org/json/json/20240303/json-20240303.jar
  echo "0ebb88e8fb5122b7506d5cf1d69f1ccdb790d22a  json.jar" | sha1sum -c --quiet
fi
cargo build -q --release
javac -cp json.jar Bench.java
java -Djava.library.path=target/release -cp json.jar:. Bench
