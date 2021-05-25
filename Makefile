out/usr/local/lib64/libthreescalers.so:
	cargo cinstall --destdir=out --prefix=/usr/local --libdir=/usr/local/lib64

.PHONY: so-build
so-build: out/usr/local/lib64/libthreescalers.so

.PHONY: so-install
so-install: out/usr/local/lib64/libthreescalers.so
	sudo chown -R root: out
	sudo cp -av out/* /

.PHONY: so-clean
so-clean:
	sudo rm -rf /usr/local/lib64/libthreescalers*
	sudo rm -rf /usr/local/lib64/pkgconfig/threescalers.pc
	-sudo rmdir /usr/local/lib64/pkgconfig
	sudo rm -rf /usr/local/include/threescalers
	-sudo rmdir /usr/local/include
