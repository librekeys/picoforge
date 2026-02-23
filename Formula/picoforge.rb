class Picoforge < Formula
  desc "An open source commissioning tool for Pico FIDO security keys"
  homepage "https://github.com/librekeys/picoforge"
  url "https://github.com/librekeys/picoforge/archive/aefd02c5e3c7877e88105c406455f56331c78995.tar.gz"
  version "0.4.0"
  sha256 "6ba7d3f6a200be7f1b3a974b89a54480957a0ebc9da08af7cf4f71cf10bd18fb"
  license "AGPL-3.0-only"

  depends_on xcode: :build
  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/picoforge", "--help"
  end
end
