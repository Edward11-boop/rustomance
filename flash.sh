#!/bin/bash
openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
-c "adapter speed 480" \
-c "reset_config srst_only" \
-c "program $1 verify reset exit"