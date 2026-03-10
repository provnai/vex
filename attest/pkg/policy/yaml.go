package policy

import (
	"gopkg.in/yaml.v3"
)

func ExportToYAML(p *Policy) string {
	data, err := yaml.Marshal(p)
	if err != nil {
		return ""
	}
	return string(data)
}

func LoadPolicyFromBytes(data []byte) (*Policy, error) {
	var p Policy
	if err := yaml.Unmarshal(data, &p); err != nil {
		return nil, err
	}
	return &p, nil
}
