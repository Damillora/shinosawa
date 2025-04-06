# the shinosawa memory layout

|Start                  |User/Kernel|Used For      |
|-----------------------|-----------|--------------|
|`0x0000_0000_0020_0000`|User       |User code     |
|`0x0000_0380_0000_0000`|User       |User stack    |
|`0x0000_4444_0000_0000`|Kernel     |ACPI handler  |
|`0x0000_4444_4444_0000`|Kernel     |Kernel heap   |
|`0xffff_ffff_8000_0000`|Kernel     |HHDM (limine) |