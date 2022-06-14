/usr/bin/time -v ../transport-binaries/transport-client-slow 127.0.0.1 40001 template-1 15034 > /dev/null
/usr/bin/time -v target/release/transport 127.0.0.1 40001 solution-1 15034 > /dev/null

/usr/bin/time -v ../transport-binaries/transport-client-fast 127.0.0.1 40001 template-3 1000000 > /dev/null
/usr/bin/time -v target/release/transport 127.0.0.1 40001 solution-3 1000000 > /dev/null

/usr/bin/time -v ../transport-binaries/transport-client-fast 127.0.0.1 40001 template-4 9000000 > /dev/null
/usr/bin/time -v target/release/transport 127.0.0.1 40001 solution-4 9000000 > /dev/null
