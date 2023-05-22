#!/bin/bash

curl --fail --silent \
  'https://monitoring.nixos.org/prometheus/api/v1/query?query=channel_revision' \
  | jq -r '{ "allowed_branches":  [(.data.result[] | select(.metric.current == "1") | .metric.channel)] | sort }'
