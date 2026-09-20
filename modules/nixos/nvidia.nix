# Headless NVIDIA compute (CUDA), driven off a GPU passed through by the
# hypervisor — no X server. `videoDrivers` must list "nvidia" even headless;
# that's what gates the nixpkgs nvidia module, not xserver.enable.
# `nvidiaPackages.latest` + open kernel modules track Blackwell support,
# which lands well ahead of the "stable"/"production" branches.
{ config, ... }:
{
  services.xserver.videoDrivers = [ "nvidia" ];

  hardware.nvidia = {
    package = config.boot.kernelPackages.nvidiaPackages.latest;
    open = true;
    modesetting.enable = true;
    nvidiaSettings = false;
  };

  # Generates the CDI spec so container runtimes (Docker's `gpus:` compose
  # key, `docker run --gpus`) can discover the GPU. Inert without a container
  # runtime; harmless to enable on every nvidia host.
  hardware.nvidia-container-toolkit.enable = true;
}
