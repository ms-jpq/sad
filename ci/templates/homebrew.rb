# frozen_string_literal: true

class Sad < Formula
  desc '{{ desc }}'
  homepage '{{ repo }}'
  version '{{ version }}'

  if Hardware::CPU.arm?
    url '{{ aarch64_uri }}'
    sha256 '{{ aarch64_sha }}'
  end

  if Hardware::CPU.intel?
    url '{{ x86_uri }}'
    sha256 '{{ x86_sha }}'
  end

  depends_on 'fzf'
  depends_on 'git-delta'

  def install
    bin.install 'sad'
  end
end
