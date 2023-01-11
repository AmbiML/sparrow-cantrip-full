# Sparrow-specific configuration.

# Carefully size the rootserver data. Peak memory use during boot is when
# the rootserver runs so we tune this and the rootserver's internal data
# structure sizes to minimize waste.
if (RELEASE)
  set(KernelRootCNodeSizeBits 11 CACHE STRING "Root CNode Size (2^n slots)")
  set(KernelMaxNumBootinfoUntypedCaps 128 CACHE STRING "Max number of bootinfo untyped caps")
else()
  # NB: for Sparrow, 13 works but is tight
  set(KernelRootCNodeSizeBits 13 CACHE STRING "Root CNode Size (2^n slots)")
  set(KernelMaxNumBootinfoUntypedCaps 128 CACHE STRING "Max number of bootinfo untyped caps")
endif()
